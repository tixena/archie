//! A bundle made from a tree, a file or a zip: the files it carries, the path each target
//! names, the walks over its embeds, and its errors.

use alloc::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;

use os_shim::System as _;
use os_shim::archive::{ZipOptions, ZipStreamWriter, to_zip_writer};
use os_shim::mock::MemorySystem;

use super::{
    BundledDocument, BundledDocumentError, Embed, Missing, embedded_paths, embeds, images_mut,
    path_of, read,
};
use crate::parser::ast::LinkForm;
use crate::parser::lexer::BLOCK_CAP;
use crate::parser::parse;
use crate::writer::{WriteOptions, write};

/// The bytes of a test picture.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n not a real picture";

/// A picture in a heading, a block quote, a quote inside a quote, a list item, a table cell, a
/// link's text and emphasis, a wiki embed, an HTML picture and a code block.
const PLACES: &str = "# Head ![h](media/h.png)

> quoted ![q](media/q.png)
>
> > nested ![n](media/n.png)

- item ![i](media/i.png)

| a | b |
|---|---|
| ![t](media/t.png) | x |

[![badge](media/b.png)](https://ci)

*em ![e](media/e.png)*

![[note#Heading|alt]]

<img src=\"media/html.png\">

```
![c](media/code.png)
```
";

/// The targets of `PLACES`, in document order.
const TARGETS: [&str; 8] = [
    "media/h.png",
    "media/q.png",
    "media/n.png",
    "media/i.png",
    "media/t.png",
    "media/b.png",
    "media/e.png",
    "note#Heading",
];

/// `pairs` as the files a bundle is made from.
fn files(pairs: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    pairs
        .iter()
        .map(|(name, bytes)| ((*name).to_owned(), bytes.to_vec()))
        .collect()
}

/// The names the central directory of `zip` lists, in its order.
fn entries(zip: &[u8]) -> Vec<String> {
    let end = zip.len() - 22;
    assert_eq!(&zip[end..end + 4], b"PK\x05\x06");
    let count = number(&zip[end + 10..end + 12]);
    let mut at = number(&zip[end + 16..end + 20]);
    let mut names = Vec::new();
    for _ in 0..count {
        assert_eq!(&zip[at..at + 4], b"PK\x01\x02");
        let length = number(&zip[at + 28..at + 30]);
        let extra = number(&zip[at + 30..at + 32]);
        let comment = number(&zip[at + 32..at + 34]);
        names.push(String::from_utf8(zip[at + 46..at + 46 + length].to_vec()).unwrap());
        at += 46 + length + extra + comment;
    }
    names
}

/// The number `bytes` write, least significant byte first.
fn number(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .rev()
        .fold(0, |value, byte| (value << 8_u32) | usize::from(*byte))
}

/// The zip `system` holds at its root, as `to_zip_writer` writes it.
fn zip_of(system: &MemorySystem) -> Vec<u8> {
    let mut zip = Vec::new();
    to_zip_writer(system, Path::new("/"), &mut zip, ZipOptions::default()).unwrap();
    zip
}

#[test]
fn the_sample_round_trips_through_a_pipe() {
    let document = parse("# Report\n\n![Logo](media/logo.png)\n").document;
    let given = files(&[("media/logo.png", PNG)]);
    let bundle = BundledDocument::new(document.clone(), given, Missing::Refuse).unwrap();
    let mut zip = Vec::new();
    bundle.to_zip(&mut zip, &WriteOptions::default()).unwrap();
    let back = BundledDocument::from_zip(&mut zip.as_slice(), Missing::Refuse).unwrap();
    assert_eq!(back.files(), ["media/logo.png"]);
    assert_eq!(back.file("media/logo.png"), Some(PNG));
    assert_eq!(back.document(), &document);
    assert_eq!(back, bundle);
}

#[test]
fn the_zip_of_the_sample_lists_its_folder_its_index_and_its_picture() {
    let document = parse("# Report\n\n![Logo](media/logo.png)\n").document;
    let given = files(&[("media/logo.png", PNG)]);
    let bundle = BundledDocument::new(document, given, Missing::Refuse).unwrap();
    let mut zip = Vec::new();
    bundle.to_zip(&mut zip, &WriteOptions::default()).unwrap();
    assert_eq!(entries(&zip), ["media/", "index.md", "media/logo.png"]);
}

#[test]
fn a_target_maps_to_its_file_path() {
    for (target, path) in [
        ("media/logo.png", "media/logo.png"),
        ("./media/logo.png", "media/logo.png"),
        ("media/../media/logo.png", "media/logo.png"),
        ("media/my%20logo.png", "media/my logo.png"),
        ("./media/logo.png?v=2#x", "media/logo.png"),
        ("dark.png#gh-dark-mode-only", "dark.png"),
        ("media/c%23.png", "media/c#.png"),
    ] {
        assert_eq!(path_of(target).unwrap().as_deref(), Some(path), "{target}");
    }
    for target in [
        "https://example.com/logo.png",
        "data:image/png;base64,iVBORw0KGgo=",
        "/srv/logo.png",
        "//cdn.example.com/logo.png",
        "C:/logo.png",
        "#top",
    ] {
        assert_eq!(path_of(target).unwrap(), None, "{target}");
    }
    for target in ["../logo.png", "media/../../logo.png", "%2E%2E/logo.png"] {
        assert!(
            matches!(
                path_of(target),
                Err(BundledDocumentError::Escapes { target: named }) if named == target
            ),
            "{target}"
        );
    }
    assert!(matches!(
        path_of("media/%FF.png"),
        Err(BundledDocumentError::Missing { target }) if target == "media/%FF.png"
    ));
}

#[test]
fn a_wiki_target_and_a_note_s_target_are_paths_from_the_folder_of_the_text_that_holds_them() {
    let document = parse("![[logo.png|100]]\n\n![[sub/note#Heading]]\n").document;
    assert_eq!(
        embedded_paths(&document, "").unwrap(),
        [
            ("logo.png".to_owned(), "logo.png".to_owned()),
            ("sub/note#Heading".to_owned(), "sub/note.md".to_owned()),
        ]
    );
    let note = parse("![chart](../media/chart.png)\n").document;
    assert_eq!(
        embedded_paths(&note, "parts").unwrap(),
        [(
            "../media/chart.png".to_owned(),
            "media/chart.png".to_owned()
        )]
    );
}

#[test]
fn every_kind_of_place_holds_a_picture() {
    let found = embeds(&parse(PLACES).document);
    let targets: Vec<&str> = found.iter().map(|embed| embed.target.as_str()).collect();
    assert_eq!(targets, TARGETS);
    let forms: Vec<LinkForm> = found.iter().map(|embed| embed.form).collect();
    assert_eq!(forms[..7], [LinkForm::Markdown; 7]);
    assert_eq!(forms[7], LinkForm::Wiki);
}

#[test]
fn each_image_is_given_in_document_order_and_only_a_quote_whose_images_change_is_written_again() {
    let source = format!("{PLACES}\n> * star\n");
    let mut document = parse(&source).document;
    let mut seen = Vec::new();
    images_mut(&mut document, &mut |image| {
        seen.push(image.target.clone());
        if image.target == "media/n.png" {
            "media/m.png".clone_into(&mut image.target);
        }
    })
    .unwrap();
    assert_eq!(seen, TARGETS);
    let written = write(&document, &WriteOptions::default()).unwrap();
    assert!(
        written.contains("> > nested ![n](media/m.png)\n"),
        "{written}"
    );
    assert!(written.ends_with("\n> * star\n"), "{written}");
    let targets: Vec<String> = embeds(&document)
        .into_iter()
        .map(|embed| embed.target)
        .collect();
    assert_eq!(targets[2], "media/m.png");
}

#[test]
fn every_embed_travels_and_a_link_does_not() {
    let document = parse(
        "![[logo.png|100]]\n\n![[sub/note#Heading]]\n\n![d](dark.png#gh-dark-mode-only)\n\n\
         [spec](docs/spec.pdf)\n",
    )
    .document;
    let given = files(&[
        ("dark.png", b"dark".as_slice()),
        ("logo.png", PNG),
        (
            "sub/note.md",
            b"# Heading\n\n![p](pic.png)\n\n[other](other.md)\n".as_slice(),
        ),
        ("sub/pic.png", b"pic".as_slice()),
    ]);
    let bundle = BundledDocument::new(document, given, Missing::Refuse).unwrap();
    assert_eq!(
        bundle.files(),
        ["dark.png", "logo.png", "sub/note.md", "sub/pic.png"]
    );
}

#[test]
fn a_note_that_embeds_itself_is_read_once() {
    let document = parse("![[self]]\n").document;
    let given = files(&[("self.md", b"![[self]]\n".as_slice())]);
    let bundle = BundledDocument::new(document, given, Missing::Refuse).unwrap();
    assert_eq!(bundle.files(), ["self.md"]);
}

#[test]
fn a_note_on_disk_travels_with_its_pictures() {
    let system = MemorySystem::new()
        .with_file("/notes/report.md", b"# Report\n\n![[parts/intro]]\n")
        .unwrap()
        .with_file("/notes/parts/intro.md", b"![chart](../media/chart.png)\n")
        .unwrap()
        .with_file("/notes/media/chart.png", PNG)
        .unwrap();
    let path = Path::new("/notes/report.md");
    let bundle = BundledDocument::from_file(&system, path, Missing::Refuse).unwrap();
    assert_eq!(bundle.files(), ["media/chart.png", "parts/intro.md"]);
    assert_eq!(bundle.file("media/chart.png"), Some(PNG));
    let mut zip = Vec::new();
    bundle.to_zip(&mut zip, &WriteOptions::default()).unwrap();
    let back = BundledDocument::from_zip(&mut zip.as_slice(), Missing::Refuse).unwrap();
    assert_eq!(back, bundle);
}

#[test]
fn a_named_file_that_is_not_given_fails_naming_its_target() {
    let document = parse("![Logo](media/my%20logo.png)\n").document;
    let error =
        BundledDocument::new(document.clone(), BTreeMap::new(), Missing::Refuse).unwrap_err();
    assert_eq!(
        error.to_string(),
        "index.md names media/my%20logo.png, and the bundle does not hold it"
    );
    let given = files(&[("media/my logo.png", PNG)]);
    let bundle = BundledDocument::new(document, given, Missing::Refuse).unwrap();
    assert_eq!(bundle.files(), ["media/my logo.png"]);
}

#[test]
fn a_target_above_the_folder_fails() {
    let document = parse("![Logo](../logo.png)\n").document;
    let error = BundledDocument::new(document, BTreeMap::new(), Missing::Keep).unwrap_err();
    assert_eq!(
        error.to_string(),
        "index.md names ../logo.png, which leaves the bundle's folder"
    );
    assert!(matches!(error, BundledDocumentError::Escapes { target } if target == "../logo.png"));
}

#[test]
fn a_file_the_document_does_not_name_is_left_out() {
    let document = parse("![Logo](media/logo.png)\n").document;
    let given = files(&[
        ("media/logo.png", PNG),
        ("media/unused.png", b"unused".as_slice()),
    ]);
    let bundle = BundledDocument::new(document, given, Missing::Refuse).unwrap();
    assert_eq!(bundle.files(), ["media/logo.png"]);
    assert_eq!(bundle.file("media/unused.png"), None);
}

#[test]
fn a_name_outside_the_bundle_is_refused() {
    for name in ["../x.png", "/x.png", "./x.png", "a//x.png", "index.md"] {
        let document = parse("![x](x.png)\n").document;
        let result = BundledDocument::new(document, files(&[(name, PNG)]), Missing::Keep);
        assert!(
            matches!(
                result,
                Err(BundledDocumentError::NotRelative { name: refused }) if refused == name
            ),
            "{name}"
        );
    }
}

#[test]
fn a_file_and_its_pictures_make_a_bundle_and_back() {
    let system = MemorySystem::new()
        .with_file("/notes/report.md", b"# Report\n\n![Logo](media/logo.png)\n")
        .unwrap()
        .with_file("/notes/media/logo.png", PNG)
        .unwrap()
        .with_file("/notes/media/other.png", b"other")
        .unwrap();
    let path = Path::new("/notes/report.md");
    let bundle = BundledDocument::from_file(&system, path, Missing::Refuse).unwrap();
    assert_eq!(bundle.files(), ["media/logo.png"]);
    let copy = Path::new("/out/copy.md");
    bundle
        .to_file(&system, copy, &WriteOptions::default())
        .unwrap();
    assert_eq!(
        system.read_to_string(copy).unwrap(),
        "# Report\n\n![Logo](media/logo.png)\n"
    );
    assert_eq!(
        read(&system, Path::new("/out/media/logo.png")).unwrap(),
        PNG
    );
    assert!(!system.exists(Path::new("/out/media/other.png")).unwrap());
}

#[test]
fn a_file_whose_picture_is_absent_fails() {
    let system = MemorySystem::new()
        .with_file("/notes/report.md", b"![Logo](media/logo.png)\n")
        .unwrap();
    let path = Path::new("/notes/report.md");
    assert!(matches!(
        BundledDocument::from_file(&system, path, Missing::Refuse),
        Err(BundledDocumentError::Missing { target }) if target == "media/logo.png"
    ));
}

#[test]
fn a_missing_file_is_refused_unless_it_is_allowed() {
    let document = parse("![Logo](media/logo.png)\n").document;
    assert!(matches!(
        BundledDocument::new(document.clone(), BTreeMap::new(), Missing::Refuse),
        Err(BundledDocumentError::Missing { target }) if target == "media/logo.png"
    ));
    let kept = BundledDocument::new(document.clone(), BTreeMap::new(), Missing::Keep).unwrap();
    assert_eq!(kept.files(), Vec::<&str>::new());
    assert_eq!(kept.document(), &document);
    let system = MemorySystem::new()
        .with_file("/notes/a.md", b"![Logo](media/logo.png)\n")
        .unwrap();
    let read_back =
        BundledDocument::from_file(&system, Path::new("/notes/a.md"), Missing::Keep).unwrap();
    read_back
        .to_file(&system, Path::new("/out/a.md"), &WriteOptions::default())
        .unwrap();
    assert_eq!(
        system.read_to_string(Path::new("/out/a.md")).unwrap(),
        "![Logo](media/logo.png)\n"
    );
}

#[test]
fn a_target_that_is_not_utf8_once_decoded_is_refused_unless_it_is_allowed() {
    let document = parse("![x](media/%FF.png)\n\n![y](media/logo.png)\n").document;
    let given = files(&[("media/logo.png", PNG)]);
    assert!(matches!(
        BundledDocument::new(document.clone(), given.clone(), Missing::Refuse),
        Err(BundledDocumentError::Missing { target }) if target == "media/%FF.png"
    ));
    let kept = BundledDocument::new(document.clone(), given, Missing::Keep).unwrap();
    assert_eq!(kept.files(), ["media/logo.png"]);
    assert_eq!(kept.document(), &document);
}

#[test]
fn a_target_that_is_not_utf8_once_decoded_has_no_embedded_path() {
    let document = parse("![x](media/%FF.png)\n\n![y](media/logo.png)\n").document;
    assert_eq!(
        embedded_paths(&document, "").unwrap(),
        [("media/logo.png".to_owned(), "media/logo.png".to_owned())]
    );
}

#[test]
fn a_note_s_target_that_is_not_utf8_once_decoded_is_refused_unless_it_is_allowed() {
    let system = MemorySystem::new()
        .with_file("/notes/report.md", b"![[parts/intro]]\n")
        .unwrap()
        .with_file(
            "/notes/parts/intro.md",
            b"![x](../media/%FF.png)\n\n![y](../media/logo.png)\n",
        )
        .unwrap()
        .with_file("/notes/media/logo.png", PNG)
        .unwrap();
    let path = Path::new("/notes/report.md");
    assert!(matches!(
        BundledDocument::from_file(&system, path, Missing::Refuse),
        Err(BundledDocumentError::Missing { target }) if target == "../media/%FF.png"
    ));
    let kept = BundledDocument::from_file(&system, path, Missing::Keep).unwrap();
    assert_eq!(kept.files(), ["media/logo.png", "parts/intro.md"]);
}

#[test]
fn a_zip_with_every_entry_under_one_folder_reads() {
    let system = MemorySystem::new()
        .with_file("/report/index.md", b"![Logo](media/logo.png)\n")
        .unwrap()
        .with_file("/report/media/logo.png", PNG)
        .unwrap();
    let zip = zip_of(&system);
    assert!(entries(&zip).iter().all(|name| name.starts_with("report/")));
    let bundle = BundledDocument::from_zip(&mut zip.as_slice(), Missing::Refuse).unwrap();
    assert_eq!(bundle.files(), ["media/logo.png"]);
    assert_eq!(bundle.file("media/logo.png"), Some(PNG));
}

#[test]
fn a_zip_whose_sizes_follow_the_data_reads() {
    let mut writer = ZipStreamWriter::new(Vec::new());
    writer.start_entry("index.md").unwrap();
    writer.write_all(b"![Logo](media/logo.png)\n").unwrap();
    writer.start_entry("media/logo.png").unwrap();
    writer.write_all(PNG).unwrap();
    let zip = writer.finish().unwrap();
    assert!(MemorySystem::from_zip_stream(&mut zip.as_slice()).is_err());
    let bundle = BundledDocument::from_zip(&mut zip.as_slice(), Missing::Refuse).unwrap();
    assert_eq!(bundle.files(), ["media/logo.png"]);
    assert_eq!(bundle.file("media/logo.png"), Some(PNG));
}

#[test]
fn a_zip_with_no_index_fails() {
    let system = MemorySystem::new()
        .with_file("/media/logo.png", PNG)
        .unwrap();
    let zip = zip_of(&system);
    assert!(matches!(
        BundledDocument::from_zip(&mut zip.as_slice(), Missing::Refuse),
        Err(BundledDocumentError::NoIndex)
    ));
}

#[test]
fn quotes_nested_deeper_than_the_block_cap_run_through_both_walks() {
    let marks = "> ".repeat(BLOCK_CAP + 60);
    let mut document = parse(&format!("{marks}![q](media/q.png)\n")).document;
    assert_eq!(
        embeds(&document),
        [Embed {
            form: LinkForm::Markdown,
            target: "media/q.png".to_owned(),
        }]
    );
    images_mut(&mut document, &mut |image| {
        "media/r.png".clone_into(&mut image.target);
    })
    .unwrap();
    assert_eq!(
        write(&document, &WriteOptions::default()).unwrap(),
        format!("{marks}![q](media/r.png)\n")
    );
}
