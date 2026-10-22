//! A markdown document with every file it embeds, as one unit, and its forms on disk.

use alloc::collections::BTreeMap;
use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};

use os_shim::System;
use os_shim::archive::{ZipOptions, to_zip_writer};
use os_shim::mock::MemorySystem;

use crate::parser::ast::{
    Block, Image, Inline, LinkForm, LinkInline, MarkdownBlock, RawDocument, Section,
};
use crate::parser::parse;
use crate::writer::{self, WriteError, WriteOptions};

/// A markdown document and every file it embeds, an embedded note's own embeds included,
/// each at its path from the document's folder.
#[derive(Debug, Clone, PartialEq)]
pub struct BundledDocument {
    document: RawDocument,
    files: BTreeMap<String, Vec<u8>>,
}

/// Why a bundle could not be made, read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BundledDocumentError {
    #[error("index.md names {target}, which leaves the bundle's folder")]
    Escapes { target: String },
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("index.md names {target}, and the bundle does not hold it")]
    Missing { target: String },
    #[error("the bundle holds no index.md")]
    NoIndex,
    #[error("{name} is not a path inside the bundle")]
    NotRelative { name: String },
    #[error("{path} is not UTF-8 at byte {offset}")]
    NotUtf8 { path: String, offset: usize },
    #[error(transparent)]
    Write(#[from] WriteError),
}

/// What a bundle does with an embed whose file is not there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Missing {
    /// Carry no file for it, and keep its target as written.
    Keep,
    /// Fail with `BundledDocumentError::Missing`.
    Refuse,
}

/// An embed as written: `![alt](target)` or `![[target]]`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Embed {
    pub form: LinkForm,
    pub target: String,
}

/// What `found_in` finds: an embed, or the text of a block quote, which the tree keeps unread.
enum Found {
    Embed(Embed),
    Quote(String),
}

/// A node `found_in` has still to walk.
enum Node<'tree> {
    Block(&'tree Block),
    Body(&'tree MarkdownBlock),
    Inline(&'tree Inline),
    LinkInline(&'tree LinkInline),
    Section(&'tree Section),
}

/// A node `places_mut` has still to walk.
enum NodeMut<'tree> {
    Block(&'tree mut Block),
    Body(&'tree mut MarkdownBlock),
    Inline(&'tree mut Inline),
    LinkInline(&'tree mut LinkInline),
    Section(&'tree mut Section),
}

/// What `places_mut` finds: an image, or the text of a block quote.
enum Place<'tree> {
    Image(&'tree mut Image),
    Quote(&'tree mut String),
}

/// A block quote's text as parsed, and the tree `images_mut` changes.
struct Quote {
    before: RawDocument,
    document: RawDocument,
}

/// What an embed's target names, before it is resolved from the folder of the text that holds it.
enum Relative {
    /// A web address, a data URI, an absolute path or no path at all.
    NoFile,
    /// A path that is not UTF-8 once percent-decoded: a file no bundle can hold.
    NotText,
    Path(String),
}

impl BundledDocument {
    /// The document's file in a zip.
    pub const INDEX: &'static str = "index.md";

    #[must_use]
    pub const fn document(&self) -> &RawDocument {
        &self.document
    }

    /// The bytes of the file at `path`, as `files` lists it.
    #[must_use]
    pub fn file(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    /// The paths of the files, sorted.
    #[must_use]
    pub fn files(&self) -> Vec<&str> {
        self.files.keys().map(String::as_str).collect()
    }

    /// The markdown file at `path` in `system`, and every file it embeds, read from the file's
    /// folder.
    ///
    /// # Errors
    ///
    /// A file that does not read or is not UTF-8, and the errors of `new`.
    pub fn from_file<S>(
        system: &S,
        path: &Path,
        missing: Missing,
    ) -> Result<Self, BundledDocumentError>
    where
        S: System + ?Sized,
    {
        let document = parse(&text(&path.to_string_lossy(), read(system, path)?)?).document;
        let folder = path.parent().unwrap_or_else(|| Path::new(""));
        let present = |name: &str| -> Result<Option<Vec<u8>>, BundledDocumentError> {
            let file = folder.join(name);
            if system.is_file(&file)? {
                Ok(Some(read(system, &file)?))
            } else {
                Ok(None)
            }
        };
        let mut files = BTreeMap::new();
        for name in named(&document, present)?
            .into_iter()
            .filter_map(|(_, file)| file)
        {
            if let Some(bytes) = present(&name)? {
                files.insert(name, bytes);
            }
        }
        Self::new(document, files, missing)
    }

    /// A bundle from its zip, read whole from `source`; a pipe will do.
    ///
    /// # Errors
    ///
    /// `NoIndex` when neither the zip's root nor its one top folder holds `index.md`, a zip that
    /// does not read, and the errors of `from_file`.
    pub fn from_zip(source: &mut dyn Read, missing: Missing) -> Result<Self, BundledDocumentError> {
        let mut buffer = Vec::new();
        source.read_to_end(&mut buffer)?;
        let system = MemorySystem::from_zip_reader(Cursor::new(buffer))?;
        let root = Path::new("/");
        let folder = if system.is_file(&root.join(Self::INDEX))? {
            root.to_path_buf()
        } else {
            match system.read_dir(root)?.as_slice() {
                [only] if system.is_file(&only.join(Self::INDEX))? => only.clone(),
                _ => return Err(BundledDocumentError::NoIndex),
            }
        };
        Self::from_file(&system, &folder.join(Self::INDEX), missing)
    }

    /// `document` and the files of `files` it embeds, an embedded note's own embeds included.
    ///
    /// # Errors
    ///
    /// `NotRelative` for a name of `files` that is not a path inside the bundle, `Escapes` for a
    /// target that leaves the document's folder, `Missing` for an embedded file `files` does not
    /// hold when `missing` is `Missing::Refuse`, and `NotUtf8` for an embedded note.
    pub fn new(
        document: RawDocument,
        mut files: BTreeMap<String, Vec<u8>>,
        missing: Missing,
    ) -> Result<Self, BundledDocumentError> {
        if let Some(name) = files.keys().find(|name| !is_inside(name)) {
            return Err(BundledDocumentError::NotRelative { name: name.clone() });
        }
        let named = named(&document, |path| Ok(files.get(path).cloned()))?;
        if missing == Missing::Refuse
            && let Some((target, _)) = named
                .iter()
                .find(|(_, path)| path.as_ref().is_none_or(|name| !files.contains_key(name)))
        {
            return Err(BundledDocumentError::Missing {
                target: target.clone(),
            });
        }
        files.retain(|path, _| named.iter().any(|(_, kept)| kept.as_ref() == Some(path)));
        Ok(Self { document, files })
    }

    /// The tree written at `path` in `system` with `options`, and each file at its path from
    /// that folder.
    ///
    /// # Errors
    ///
    /// The writer's error, and a folder or a file that cannot be written.
    pub fn to_file<S>(
        &self,
        system: &S,
        path: &Path,
        options: &WriteOptions,
    ) -> Result<(), BundledDocumentError>
    where
        S: System + ?Sized,
    {
        let folder = path.parent().unwrap_or_else(|| Path::new(""));
        system.create_dir_all(folder)?;
        system.write(path, writer::write(&self.document, options)?.as_bytes())?;
        for (name, bytes) in &self.files {
            let file: PathBuf = folder.join(name);
            if let Some(parent) = file.parent() {
                system.create_dir_all(parent)?;
            }
            system.write(&file, bytes)?;
        }
        Ok(())
    }

    /// The bundle as a zip, written to `sink`: `index.md` and each file at its path.
    ///
    /// # Errors
    ///
    /// The writer's error, and a sink that cannot be written.
    pub fn to_zip(
        &self,
        sink: &mut dyn Write,
        options: &WriteOptions,
    ) -> Result<(), BundledDocumentError> {
        let system = MemorySystem::new();
        self.to_file(&system, &Path::new("/").join(Self::INDEX), options)?;
        to_zip_writer(&system, Path::new("/"), sink, ZipOptions::default())?;
        Ok(())
    }
}

/// The path of the file `target`, a `![alt](target)` target, names from the document's folder,
/// or `None` when it names none.
///
/// # Errors
///
/// `Escapes` for a path that leaves the folder, and `Missing` for one that is not UTF-8 once
/// percent-decoded.
pub fn path_of(target: &str) -> Result<Option<String>, BundledDocumentError> {
    let embed = Embed {
        form: LinkForm::Markdown,
        target: target.to_owned(),
    };
    match relative_of(&embed) {
        Relative::NoFile => Ok(None),
        Relative::NotText => Err(BundledDocumentError::Missing {
            target: target.to_owned(),
        }),
        Relative::Path(relative) => joined("", &relative, target),
    }
}

/// Each embed of `document`, `![alt](target)` and `![[target]]`, a block quote's included, in
/// document order.
#[must_use]
pub fn embeds(document: &RawDocument) -> Vec<Embed> {
    let mut embeds = Vec::new();
    let mut pending: Vec<Found> = found_in(document).into_iter().rev().collect();
    while let Some(found) = pending.pop() {
        match found {
            Found::Embed(embed) => embeds.push(embed),
            Found::Quote(quoted) => {
                pending.extend(found_in(&parse(&quoted).document).into_iter().rev());
            }
        }
    }
    embeds
}

/// Gives each image of `document`, a block quote's included, to `visit`, in document order. A
/// block quote whose images `visit` changes is written again from its changed tree.
///
/// # Errors
///
/// The writer's error for a block quote whose changed tree does not write.
pub fn images_mut(
    document: &mut RawDocument,
    visit: &mut dyn FnMut(&mut Image),
) -> Result<(), WriteError> {
    let mut levels: Vec<Vec<Quote>> = Vec::new();
    let mut quoted = quotes_in(document);
    while !quoted.is_empty() {
        let level: Vec<Quote> = quoted
            .iter()
            .map(|text| {
                let parsed = parse(text).document;
                Quote {
                    before: parsed.clone(),
                    document: parsed,
                }
            })
            .collect();
        quoted = level
            .iter()
            .flat_map(|quote| quotes_in(&quote.document))
            .collect();
        levels.push(level);
    }
    visit_in_order(document, &mut levels, visit);
    write_back(document, &mut levels)
}

/// Each file that `document`, the text at `folder`, embeds, as the target as written and the
/// file's path in the bundle, each path once, in document order.
///
/// An embedded note's own embeds are not followed, and a target that is not UTF-8 once
/// percent-decoded is left out.
///
/// # Errors
///
/// `Escapes` for a target that leaves the bundle's folder.
pub fn embedded_paths(
    document: &RawDocument,
    folder: &str,
) -> Result<Vec<(String, String)>, BundledDocumentError> {
    let mut paths: Vec<(String, String)> = Vec::new();
    for (target, file) in paths_in(document, folder)? {
        if let Some(path) = file
            && !paths.iter().any(|(_, known)| *known == path)
        {
            paths.push((target, path));
        }
    }
    Ok(paths)
}

/// Each embed of `document`, the text at `folder`, that names a file, as the target as written
/// and the file's path in the bundle, `None` for a file no bundle can hold, in document order.
fn paths_in(
    document: &RawDocument,
    folder: &str,
) -> Result<Vec<(String, Option<String>)>, BundledDocumentError> {
    let mut paths = Vec::new();
    for embed in embeds(document) {
        let path = match relative_of(&embed) {
            Relative::NoFile => continue,
            Relative::NotText => None,
            Relative::Path(relative) => {
                let Some(path) = joined(folder, &relative, &embed.target)? else {
                    continue;
                };
                Some(path)
            }
        };
        paths.push((embed.target, path));
    }
    Ok(paths)
}

/// Every file that `document` embeds, an embedded note's own embeds included, each path once, as
/// the target as written and the file's path in the bundle, `None` for a file no bundle can hold.
/// `text_of` gives an embedded note's bytes, or `None` when it is not there.
fn named<F>(
    document: &RawDocument,
    mut text_of: F,
) -> Result<Vec<(String, Option<String>)>, BundledDocumentError>
where
    F: FnMut(&str) -> Result<Option<Vec<u8>>, BundledDocumentError>,
{
    let mut named: Vec<(String, Option<String>)> = Vec::new();
    let mut pending: Vec<(String, RawDocument)> = vec![(String::new(), document.clone())];
    while let Some((folder, note)) = pending.pop() {
        for (target, file) in paths_in(&note, &folder)? {
            if file.is_some() && named.iter().any(|(_, known)| *known == file) {
                continue;
            }
            named.push((target, file.clone()));
            if let Some(path) = file
                && Path::new(&path)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
                && let Some(bytes) = text_of(&path)?
            {
                pending.push((
                    parent_of(&path).to_owned(),
                    parse(&text(&path, bytes)?).document,
                ));
            }
        }
    }
    Ok(named)
}

/// The path an embed names, relative to the folder of the text that holds it, not yet resolved.
/// `![alt](target)` is an address: percent-decoded, its `?` and `#` parts cut off.
/// `![[target]]` is a name: kept as written, its `#` part cut off, and `<target>.md` when it has
/// no extension.
fn relative_of(embed: &Embed) -> Relative {
    let target = embed.target.as_str();
    let cut = match embed.form {
        LinkForm::Markdown => target.find(['?', '#']),
        LinkForm::Wiki => target.find('#'),
    };
    let reference = &target[..cut.unwrap_or(target.len())];
    if reference.is_empty() || reference.starts_with('/') || has_scheme(reference) {
        return Relative::NoFile;
    }
    match embed.form {
        LinkForm::Markdown => {
            let decoded = percent_decoded(reference);
            if decoded
                .utf8_chunks()
                .any(|chunk| !chunk.invalid().is_empty())
            {
                return Relative::NotText;
            }
            Relative::Path(decoded.utf8_chunks().map(|chunk| chunk.valid()).collect())
        }
        LinkForm::Wiki => {
            let named = Path::new(reference).extension().is_some();
            Relative::Path(if named {
                reference.to_owned()
            } else {
                format!("{reference}.md")
            })
        }
    }
}

/// `relative` resolved from `folder`, a path inside the bundle: empty and `.` parts dropped,
/// `..` removing the part before it, and `Escapes` naming `target` when there is none.
fn joined(
    folder: &str,
    relative: &str,
    target: &str,
) -> Result<Option<String>, BundledDocumentError> {
    let mut parts: Vec<&str> = folder.split('/').filter(|part| !part.is_empty()).collect();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(BundledDocumentError::Escapes {
                        target: target.to_owned(),
                    });
                }
            }
            name => parts.push(name),
        }
    }
    Ok((!parts.is_empty()).then(|| parts.join("/")))
}

/// Whether `reference` opens with a URL scheme: a letter, then letters, digits, `+`, `-` or
/// `.`, then `:`.
fn has_scheme(reference: &str) -> bool {
    let Some(colon) = reference.find(':') else {
        return false;
    };
    let mut characters = reference[..colon].chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|next| next.is_ascii_alphanumeric() || matches!(next, '+' | '-' | '.'))
}

/// `text` with each `%` and two hex digits replaced by the byte they write.
fn percent_decoded(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let hex = |at: usize| {
        let byte = bytes.get(at)?;
        char::from(*byte).to_digit(16)
    };
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0_usize;
    while at < bytes.len() {
        if let (b'%', Some(high), Some(low)) = (bytes[at], hex(at + 1), hex(at + 2)) {
            decoded.push(u8::try_from(high * 16 + low).unwrap_or_default());
            at += 3;
        } else {
            decoded.push(bytes[at]);
            at += 1;
        }
    }
    decoded
}

fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

/// Whether `name` is a path inside the bundle: no empty, `.` or `..` part, no leading `/`,
/// and not the zip's own `index.md`.
fn is_inside(name: &str) -> bool {
    name != BundledDocument::INDEX
        && !name.starts_with('/')
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn text(path: &str, bytes: Vec<u8>) -> Result<String, BundledDocumentError> {
    String::from_utf8(bytes).map_err(|error| BundledDocumentError::NotUtf8 {
        path: path.to_owned(),
        offset: error.utf8_error().valid_up_to(),
    })
}

fn read<S>(system: &S, path: &Path) -> io::Result<Vec<u8>>
where
    S: System + ?Sized,
{
    let mut bytes = Vec::new();
    system.open(path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The text of each block quote of `document`, in document order, a quote inside a quote left
/// in its text.
fn quotes_in(document: &RawDocument) -> Vec<String> {
    found_in(document)
        .into_iter()
        .filter_map(|found| match found {
            Found::Embed(_) => None,
            Found::Quote(quoted) => Some(quoted),
        })
        .collect()
}

/// Gives each image of `document` and of the quotes of `levels` to `visit`, in document order.
/// `levels[0]` holds the quotes of `document`, and each level the quotes of the one before it.
fn visit_in_order(
    document: &mut RawDocument,
    levels: &mut [Vec<Quote>],
    visit: &mut dyn FnMut(&mut Image),
) {
    let mut below: Vec<_> = levels
        .iter_mut()
        .map(|level| {
            level
                .iter_mut()
                .map(|quote| places_mut(&mut quote.document).into_iter())
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect();
    let mut stack = vec![(0_usize, places_mut(document).into_iter())];
    while let Some((depth, mut places)) = stack.pop() {
        let Some(place) = places.next() else {
            continue;
        };
        stack.push((depth, places));
        match place {
            Place::Image(image) => visit(image),
            // A level lists its quotes in the order a walk in document order meets them.
            Place::Quote(_) => {
                if let Some(quote) = below.get_mut(depth).and_then(Iterator::next) {
                    stack.push((depth + 1, quote));
                }
            }
        }
    }
}

/// Each quote of `levels` whose tree changed, written into the text of the block quote it was
/// read from, the deepest level first.
fn write_back(document: &mut RawDocument, levels: &mut [Vec<Quote>]) -> Result<(), WriteError> {
    for depth in (0..levels.len()).rev() {
        let (above, rest) = levels.split_at_mut(depth);
        let Some(level) = rest.first() else {
            continue;
        };
        let values: Vec<&mut String> = above.last_mut().map_or_else(
            || quote_values(document),
            |parents| {
                parents
                    .iter_mut()
                    .flat_map(|parent| quote_values(&mut parent.document))
                    .collect()
            },
        );
        for (value, quote) in values.into_iter().zip(level) {
            if quote.document != quote.before {
                writer::write(&quote.document, &WriteOptions::default())?
                    .trim_end_matches('\n')
                    .clone_into(value);
            }
        }
    }
    Ok(())
}

fn quote_values(document: &mut RawDocument) -> Vec<&mut String> {
    places_mut(document)
        .into_iter()
        .filter_map(|place| match place {
            Place::Image(_) => None,
            Place::Quote(value) => Some(value),
        })
        .collect()
}

/// The embeds and the block quotes' texts of `document`, in document order.
fn found_in(document: &RawDocument) -> Vec<Found> {
    let root = match document {
        RawDocument::Invalid(invalid) => &invalid.body,
        RawDocument::Valid(valid) => &valid.body,
    };
    let embed = |image: &Image| {
        Found::Embed(Embed {
            form: image.form,
            target: image.target.clone(),
        })
    };
    let mut found = Vec::new();
    let mut stack = vec![Node::Body(root)];
    while let Some(node) = stack.pop() {
        match node {
            Node::Block(block) => match block {
                Block::BlockQuote(quote) => found.push(Found::Quote(quote.value.clone())),
                Block::CodeBlock(_) | Block::Html(_) | Block::ThematicBreak(_) => {}
                Block::List(list) => {
                    stack.extend(list.items.iter().rev().map(|item| Node::Body(&item.body)));
                }
                Block::Paragraph(paragraph) => {
                    for line in paragraph.lines.iter().rev() {
                        stack.extend(line.children.iter().rev().map(Node::Inline));
                    }
                }
                Block::Table(table) => {
                    for row in table.head.iter().chain(&table.rows).rev() {
                        stack.extend(row.cells.iter().rev().map(|cell| Node::Body(&cell.body)));
                    }
                }
            },
            Node::Body(body) => stack.extend(body.sections.iter().rev().map(Node::Section)),
            Node::Inline(inline) => match inline {
                Inline::Code(_) | Inline::Html(_) | Inline::LineBreak(_) | Inline::Text(_) => {}
                Inline::Emphasis(emphasis) => {
                    stack.extend(emphasis.children.iter().rev().map(Node::Inline));
                }
                Inline::Image(image) => found.push(embed(image)),
                Inline::Link(link) => {
                    stack.extend(link.children.iter().rev().map(Node::LinkInline));
                }
                Inline::Strong(strong) => {
                    stack.extend(strong.children.iter().rev().map(Node::Inline));
                }
            },
            Node::LinkInline(inline) => match inline {
                LinkInline::Code(_)
                | LinkInline::Html(_)
                | LinkInline::LineBreak(_)
                | LinkInline::Text(_) => {}
                LinkInline::Emphasis(emphasis) => {
                    stack.extend(emphasis.children.iter().rev().map(Node::LinkInline));
                }
                LinkInline::Image(image) => found.push(embed(image)),
                LinkInline::Strong(strong) => {
                    stack.extend(strong.children.iter().rev().map(Node::LinkInline));
                }
            },
            Node::Section(section) => {
                stack.extend(section.subsections.iter().rev().map(Node::Section));
                stack.extend(section.blocks.iter().rev().map(Node::Block));
                if let Some(heading) = &section.heading {
                    stack.extend(heading.children.iter().rev().map(Node::Inline));
                }
            }
        }
    }
    found
}

/// The images and the block quotes' texts of `document`, in document order, to change in
/// place.
fn places_mut(document: &mut RawDocument) -> Vec<Place<'_>> {
    let root = match document {
        RawDocument::Invalid(invalid) => &mut invalid.body,
        RawDocument::Valid(valid) => &mut valid.body,
    };
    let mut places = Vec::new();
    let mut stack = vec![NodeMut::Body(root)];
    while let Some(node) = stack.pop() {
        match node {
            NodeMut::Block(block) => match block {
                Block::BlockQuote(quote) => places.push(Place::Quote(&mut quote.value)),
                Block::CodeBlock(_) | Block::Html(_) | Block::ThematicBreak(_) => {}
                Block::List(list) => {
                    stack.extend(
                        list.items
                            .iter_mut()
                            .rev()
                            .map(|item| NodeMut::Body(&mut item.body)),
                    );
                }
                Block::Paragraph(paragraph) => {
                    for line in paragraph.lines.iter_mut().rev() {
                        stack.extend(line.children.iter_mut().rev().map(NodeMut::Inline));
                    }
                }
                Block::Table(table) => {
                    for row in table.head.iter_mut().chain(table.rows.iter_mut()).rev() {
                        stack.extend(
                            row.cells
                                .iter_mut()
                                .rev()
                                .map(|cell| NodeMut::Body(&mut cell.body)),
                        );
                    }
                }
            },
            NodeMut::Body(body) => {
                stack.extend(body.sections.iter_mut().rev().map(NodeMut::Section));
            }
            NodeMut::Inline(inline) => match inline {
                Inline::Code(_) | Inline::Html(_) | Inline::LineBreak(_) | Inline::Text(_) => {}
                Inline::Emphasis(emphasis) => {
                    stack.extend(emphasis.children.iter_mut().rev().map(NodeMut::Inline));
                }
                Inline::Image(image) => places.push(Place::Image(image)),
                Inline::Link(link) => {
                    stack.extend(link.children.iter_mut().rev().map(NodeMut::LinkInline));
                }
                Inline::Strong(strong) => {
                    stack.extend(strong.children.iter_mut().rev().map(NodeMut::Inline));
                }
            },
            NodeMut::LinkInline(inline) => match inline {
                LinkInline::Code(_)
                | LinkInline::Html(_)
                | LinkInline::LineBreak(_)
                | LinkInline::Text(_) => {}
                LinkInline::Emphasis(emphasis) => {
                    stack.extend(emphasis.children.iter_mut().rev().map(NodeMut::LinkInline));
                }
                LinkInline::Image(image) => places.push(Place::Image(image)),
                LinkInline::Strong(strong) => {
                    stack.extend(strong.children.iter_mut().rev().map(NodeMut::LinkInline));
                }
            },
            NodeMut::Section(section) => {
                let Section {
                    blocks,
                    heading,
                    kind: _,
                    subsections,
                } = section;
                stack.extend(subsections.iter_mut().rev().map(NodeMut::Section));
                stack.extend(blocks.iter_mut().rev().map(NodeMut::Block));
                if let Some(title) = heading {
                    stack.extend(title.children.iter_mut().rev().map(NodeMut::Inline));
                }
            }
        }
    }
    places
}

#[cfg(test)]
mod tests;
