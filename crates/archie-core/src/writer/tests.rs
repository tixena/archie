//! Each sample is a tree written back to markdown: read from a source, or
//! built, then written, then read again.

use crate::parser::ast::{
    Block, Document, Heading, Inline, Kind, Line, MarkdownBlock, Paragraph, Section, Table, Text,
};
use crate::parser::extension::Markdown;
use crate::parser::parse;
use crate::writer::{WriteOptions, write};

/// Every file under `parser/tests/fixtures/`.
const FIXTURES: [(&str, &str); 21] = [
    (
        "api_reference",
        include_str!("../parser/tests/fixtures/api_reference.md"),
    ),
    (
        "book_page",
        include_str!("../parser/tests/fixtures/book_page.md"),
    ),
    (
        "changelog",
        include_str!("../parser/tests/fixtures/changelog.md"),
    ),
    (
        "contact",
        include_str!("../parser/tests/fixtures/contact.md"),
    ),
    (
        "daily_note",
        include_str!("../parser/tests/fixtures/daily_note.md"),
    ),
    (
        "docusaurus_page",
        include_str!("../parser/tests/fixtures/docusaurus_page.md"),
    ),
    ("essay", include_str!("../parser/tests/fixtures/essay.md")),
    (
        "hugo_post",
        include_str!("../parser/tests/fixtures/hugo_post.md"),
    ),
    (
        "hugo_post_with_table",
        include_str!("../parser/tests/fixtures/hugo_post_with_table.md"),
    ),
    (
        "issue_template",
        include_str!("../parser/tests/fixtures/issue_template.md"),
    ),
    (
        "jekyll_post",
        include_str!("../parser/tests/fixtures/jekyll_post.md"),
    ),
    (
        "meeting_notes",
        include_str!("../parser/tests/fixtures/meeting_notes.md"),
    ),
    (
        "obsidian_note",
        include_str!("../parser/tests/fixtures/obsidian_note.md"),
    ),
    (
        "pandoc_paper",
        include_str!("../parser/tests/fixtures/pandoc_paper.md"),
    ),
    ("readme", include_str!("../parser/tests/fixtures/readme.md")),
    ("recipe", include_str!("../parser/tests/fixtures/recipe.md")),
    (
        "slash_command",
        include_str!("../parser/tests/fixtures/slash_command.md"),
    ),
    ("task", include_str!("../parser/tests/fixtures/task.md")),
    (
        "toml_multi_line_string",
        include_str!("../parser/tests/fixtures/toml_multi_line_string.md"),
    ),
    (
        "yaml_list_of_mappings",
        include_str!("../parser/tests/fixtures/yaml_list_of_mappings.md"),
    ),
    (
        "zola_page",
        include_str!("../parser/tests/fixtures/zola_page.md"),
    ),
];

fn written(source: &str) -> String {
    write(&parse(source).unwrap(), &WriteOptions::default()).unwrap()
}

/// The body `source` reads as, every span emptied.
fn body(source: &str) -> MarkdownBlock {
    let mut read = parse(source).unwrap().body;
    clear_block(&mut read);
    read
}

/// Writes `source`, reads the text back, and checks that it reads as the
/// same body and writes as the same text again. Gives the text.
fn round_trip(source: &str) -> String {
    let text = written(source);
    assert_eq!(body(&text), body(source), "{source:?} became {text:?}");
    assert_eq!(written(&text), text, "{source:?} became {text:?}");
    text
}

fn clear_block(block: &mut MarkdownBlock) {
    block.span = 0..0;
    clear_kind(&mut block.kind);
    for section in &mut block.sections {
        clear_section(section);
    }
}

fn clear_kind(kind: &mut Kind<Markdown>) {
    if let Some(block) = kind.0.as_deref_mut() {
        clear_block(block);
    }
}

fn clear_section(section: &mut Section) {
    section.span = 0..0;
    clear_kind(&mut section.kind);
    section.heading.span = 0..0;
    clear_kind(&mut section.heading.kind);
    clear_inlines(&mut section.heading.children);
    for block in &mut section.blocks {
        clear_block_node(block);
    }
    for subsection in &mut section.subsections {
        clear_section(subsection);
    }
}

fn clear_block_node(block: &mut Block) {
    match block {
        Block::BlockQuote(quote) => {
            quote.span = 0..0;
            clear_kind(&mut quote.kind);
        }
        Block::CodeBlock(code) => {
            code.content = 0..0;
            code.span = 0..0;
            clear_kind(&mut code.kind);
        }
        Block::Html(html) => {
            html.span = 0..0;
            clear_kind(&mut html.kind);
        }
        Block::List(list) => {
            list.span = 0..0;
            clear_kind(&mut list.kind);
            for item in &mut list.items {
                item.span = 0..0;
                clear_kind(&mut item.kind);
                clear_block(&mut item.body);
            }
        }
        Block::Paragraph(paragraph) => {
            paragraph.span = 0..0;
            clear_kind(&mut paragraph.kind);
            for line in &mut paragraph.lines {
                line.span = 0..0;
                clear_kind(&mut line.kind);
                clear_inlines(&mut line.children);
            }
        }
        Block::Table(table) => clear_table(table),
        Block::ThematicBreak(thematic_break) => {
            thematic_break.span = 0..0;
            clear_kind(&mut thematic_break.kind);
        }
    }
}

fn clear_table(table: &mut Table) {
    table.span = 0..0;
    clear_kind(&mut table.kind);
    for row in table.head.iter_mut().chain(&mut table.rows) {
        row.span = 0..0;
        clear_kind(&mut row.kind);
        for cell in &mut row.cells {
            cell.span = 0..0;
            clear_kind(&mut cell.kind);
            clear_block(&mut cell.body);
        }
    }
}

fn clear_inlines(inlines: &mut [Inline]) {
    for inline in inlines {
        match inline {
            Inline::Code(code) => {
                code.span = 0..0;
                clear_kind(&mut code.kind);
            }
            Inline::Emphasis(emphasis) => {
                emphasis.span = 0..0;
                clear_kind(&mut emphasis.kind);
                clear_inlines(&mut emphasis.children);
            }
            Inline::Html(html) => {
                html.span = 0..0;
                clear_kind(&mut html.kind);
            }
            Inline::Image(image) => {
                image.span = 0..0;
                clear_kind(&mut image.kind);
            }
            Inline::LineBreak(line_break) => {
                line_break.span = 0..0;
                clear_kind(&mut line_break.kind);
            }
            Inline::Link(link) => {
                link.span = 0..0;
                clear_kind(&mut link.kind);
                clear_inlines(&mut link.children);
            }
            Inline::Strong(strong) => {
                strong.span = 0..0;
                clear_kind(&mut strong.kind);
                clear_inlines(&mut strong.children);
            }
            Inline::Text(text) => {
                text.span = 0..0;
                clear_kind(&mut text.kind);
            }
        }
    }
}

fn text_node(value: &str) -> Inline {
    Inline::Text(Text {
        kind: Kind(None),
        span: 0..0,
        value: value.to_owned(),
    })
}

/// A section headed `title`, holding a paragraph `text`, then
/// `subsections`.
fn titled(title: &str, text: &str, subsections: Vec<Section>) -> Section {
    Section {
        blocks: vec![Block::Paragraph(Paragraph {
            kind: Kind(None),
            lines: vec![Line {
                children: vec![text_node(text)],
                kind: Kind(None),
                span: 0..0,
            }],
            span: 0..0,
        })],
        heading: Heading {
            children: vec![text_node(title)],
            kind: Kind(None),
            span: 0..0,
        },
        kind: Kind(None),
        span: 0..0,
        subsections,
    }
}

/// Sections `Level from` to `Level to`, each inside the one before it.
fn levels(from: usize, to: usize) -> Section {
    let subsections = if from < to {
        vec![levels(from + 1, to)]
    } else {
        Vec::new()
    };
    titled(
        &format!("Level {from}"),
        &format!("text {from}"),
        subsections,
    )
}

fn document(sections: Vec<Section>) -> Document {
    Document {
        body: MarkdownBlock {
            kind: Kind(None),
            sections,
            span: 0..0,
        },
        frontmatter: None,
        kind: Kind(None),
        span: 0..0,
    }
}

/// `section` with each `markdown` block that ends its blocks, the form a
/// section seven levels deep is written in, put back among its
/// subsections.
fn lifted(mut section: Section) -> Section {
    let mut deeper = Vec::new();
    while let Some(Block::CodeBlock(code)) = section.blocks.last()
        && code.info == "markdown"
        && let Some(Block::CodeBlock(claimed)) = section.blocks.pop()
        && let Some(block) = claimed.kind.0
    {
        let mut sections = block.sections;
        sections.append(&mut deeper);
        deeper = sections;
    }
    section.subsections.append(&mut deeper);
    section.subsections = section.subsections.into_iter().map(lifted).collect();
    section
}

#[test]
fn every_fixture_reads_back_to_the_same_body_and_writes_the_same_text_again() {
    for (name, source) in FIXTURES {
        let text = written(source);
        assert_eq!(body(&text), body(source), "{name}: {text}");
        assert_eq!(written(&text), text, "{name}");
    }
}

#[test]
fn the_owners_sample_is_written_in_archies_form() {
    let source = "Notes\n=====\n\nSome __bold__ and _em_ text.\n\n* one\n*   two\n    continued\n";
    assert_eq!(
        round_trip(source),
        "# Notes\n\nSome **bold** and *em* text.\n\n- one\n- two\n  continued\n"
    );
}

#[test]
fn a_thematic_break_of_underscores_or_stars_is_written_as_dashes() {
    assert_eq!(round_trip("a\n\n___\n\n* * *\n"), "a\n\n---\n\n---\n");
}

#[test]
fn headings_get_one_mark_per_level_and_an_empty_heading_only_over_subsections() {
    assert_eq!(
        round_trip("# Hi H1\n### This is H3\n"),
        "# Hi H1\n\n##\n\n### This is H3\n"
    );
    assert_eq!(round_trip("intro\n"), "intro\n");
    assert_eq!(round_trip("intro\n## B\n"), "#\n\nintro\n\n## B\n");
    assert_eq!(
        round_trip("## Intro\n\n# Title\n"),
        "#\n\n## Intro\n\n# Title\n"
    );
    assert_eq!(round_trip("Sub\n---\ntext\n"), "#\n\n## Sub\n\ntext\n");
}

#[test]
fn a_section_seven_levels_deep_is_a_markdown_block_inside_its_parent() {
    let text = write(&document(vec![levels(1, 7)]), &WriteOptions::default()).unwrap();
    assert!(
        text.ends_with("###### Level 6\n\ntext 6\n\n```markdown\n# Level 7\n\ntext 7\n```\n"),
        "{text}"
    );
    let source = "###### F\n\n````markdown\n# G\n\n```rust\nfn main() {}\n```\n````\n";
    assert_eq!(
        round_trip(source),
        format!("#\n\n##\n\n###\n\n####\n\n#####\n\n{source}")
    );
}

#[test]
fn a_section_twenty_two_levels_deep_reads_back_to_the_same_tree() {
    let built = document(vec![levels(1, 22)]);
    let text = write(&built, &WriteOptions::default()).unwrap();
    let fences: Vec<&str> = text.lines().filter(|line| line.starts_with('`')).collect();
    assert_eq!(
        fences,
        [
            "`````markdown",
            "````markdown",
            "```markdown",
            "```",
            "````",
            "`````"
        ]
    );
    assert_eq!(written(&text), text);
    let read = parse(&text).unwrap().body;
    let mut read_back = MarkdownBlock {
        sections: read.sections.into_iter().map(lifted).collect(),
        ..read
    };
    clear_block(&mut read_back);
    assert_eq!(read_back, built.body);
}

#[test]
fn lists_are_written_with_dashes_and_numbers_counted_up_from_the_first() {
    assert_eq!(round_trip("3. a\n\n4. b\n"), "3. a\n\n4. b\n");
    assert_eq!(round_trip("1) a\n1) b\n"), "1. a\n2. b\n");
    assert_eq!(
        round_trip("+ a\n+ b\n  + c\n    more\n"),
        "- a\n- b\n  - c\n    more\n"
    );
    assert_eq!(
        round_trip("9. a\n10. b\n    more\n"),
        "9. a\n10. b\n    more\n"
    );
    assert_eq!(
        round_trip("- a\n\n  second\n- b\n"),
        "- a\n\n  second\n\n- b\n"
    );
    assert_eq!(round_trip("- # A\n  text\n- b\n"), "- # A\n  text\n- b\n");
    assert_eq!(round_trip("-\n- b\n"), "-\n- b\n");
}

#[test]
fn emphasis_strong_and_code_take_archies_marks_and_fences() {
    assert_eq!(round_trip("_a_ __b__ `c`\n"), "*a* **b** `c`\n");
    assert_eq!(round_trip("``a`b``\n"), "``a`b``\n");
    assert_eq!(
        round_trip("````\n```\ninside\n```\n````\n"),
        "````\n```\ninside\n```\n````\n"
    );
    assert_eq!(
        round_trip("text\n\n    indented\n\n    code\n"),
        "text\n\n```\nindented\n\ncode\n```\n"
    );
    assert_eq!(round_trip("```\n```\n"), "```\n```\n");
    assert_eq!(
        round_trip("- ```\n  a\n\n  b\n  ```\n"),
        "- ```\n  a\n\n  b\n  ```\n"
    );
}

#[test]
fn links_are_written_inline_and_wiki_links_and_embeds_keep_their_form() {
    assert_eq!(round_trip("[g][1]\n\n[1]: u\n"), "[g](u)\n");
    assert_eq!(
        round_trip("[a](<b c> 'd') <https://x.org>\n"),
        "[a](<b c> \"d\") [https://x.org](https://x.org)\n"
    );
    assert_eq!(
        round_trip("[[t|x]] [[only]] ![a](i.png) ![[i.png|alt]] ![[i.png]]\n"),
        "[[t|x]] [[only]] ![a](i.png) ![[i.png|alt]] ![[i.png]]\n"
    );
    assert_eq!(round_trip("[a](u(1))\n"), "[a](u(1))\n");
    assert_eq!(round_trip("[a](<u)>)\n"), "[a](<u)>)\n");
}

#[test]
fn a_node_cut_over_several_lines_is_written_once_over_the_same_lines() {
    assert_eq!(
        round_trip("Some *emphasis that\ngoes on* here.\n"),
        "Some *emphasis that\ngoes on* here.\n"
    );
    assert_eq!(
        round_trip("A [link that\ngoes on\nand on](u \"t\") here.\n"),
        "A [link that\ngoes on\nand on](u \"t\") here.\n"
    );
    assert_eq!(
        round_trip("a `code that\ngoes on` b\n"),
        "a `code that\ngoes on` b\n"
    );
    assert_eq!(
        round_trip("a ``code ` that\ngoes on`` b\n"),
        "a ``code ` that\ngoes on`` b\n"
    );
    assert_eq!(round_trip("one\ntwo\n"), "one\ntwo\n");
}

#[test]
fn an_ampersand_that_would_read_as_a_reference_is_escaped() {
    assert_eq!(
        round_trip("a &amp;amp; b &#38;#35; c & d\n"),
        "a \\&amp; b \\&#35; c & d\n"
    );
}

#[test]
fn a_code_span_that_the_reader_would_strip_is_padded() {
    assert_eq!(round_trip("`` `a` ``\n"), "`` `a` ``\n");
    assert_eq!(round_trip("`  a  `\n"), "`  a  `\n");
    assert_eq!(round_trip("` `\n"), "` `\n");
    assert_eq!(round_trip("`` a` ``\n"), "`` a` ``\n");
}

#[test]
fn text_that_would_read_as_markup_is_escaped() {
    assert_eq!(
        round_trip("\\*a\\* \\_b\\_ \\`c\\` \\[d](e) \\<f> \\!\\[g](h)\n"),
        "\\*a\\* \\_b\\_ \\`c\\` [d\\](e) \\<f> \\![g\\](h)\n"
    );
    assert_eq!(
        round_trip("snake_case a * b C:\\dir\n"),
        "snake_case a * b C:\\dir\n"
    );
    assert_eq!(
        round_trip("\\# a\n\n\\> b\n\n\\- c\n\n1\\. d\n"),
        "\\# a\n\n\\> b\n\n\\- c\n\n1\\. d\n"
    );
    assert_eq!(round_trip("a\n\\===\n"), "a\n\\===\n");
    assert_eq!(round_trip("\\[a]: b\n"), "\\[a]: b\n");
    assert_eq!(round_trip("#tag and [x] here\n"), "#tag and [x] here\n");
    assert_eq!(
        round_trip("| a \\| b |\n|---|\n"),
        "| a \\| b |\n| ------ |\n"
    );
}

#[test]
fn line_breaks_quotes_html_and_breaks_take_archies_form() {
    assert_eq!(round_trip("a  \nb\\\nc\n"), "a\\\nb\\\nc\n");
    assert_eq!(round_trip(">a\n>\n>  b\n"), "> a\n>\n>  b\n");
    assert_eq!(
        round_trip("<div>\n  <p>x</p>\n</div>\n"),
        "<div>\n  <p>x</p>\n</div>\n"
    );
    assert_eq!(round_trip("a <span>b</span>\n"), "a <span>b</span>\n");
    assert_eq!(round_trip("***\n"), "---\n");
}

#[test]
fn a_table_is_a_padded_pipe_table_or_a_grid_table() {
    assert_eq!(
        round_trip("|a|bb|\n|-|:-|\n|1|2|\n"),
        "| a   | bb  |\n| --- | :-- |\n| 1   | 2   |\n"
    );
    assert_eq!(
        round_trip("| a | b | c |\n|:-:|--:|---|\n| long cell | | x |\n"),
        "| a         | b   | c   |\n| :-------: | --: | --- |\n| long cell |     | x   |\n"
    );
    let grid = "\
+-----+-----+
| a   | b   |
+=====+====:+
| one | x   |
|     |     |
| two |     |
+-----+-----+
";
    assert_eq!(round_trip(grid), grid);
}

#[test]
fn blocks_are_separated_by_one_blank_line_and_the_text_ends_with_one_newline() {
    assert_eq!(
        round_trip("# A\ntext\n- x\n> q\n\n\n\nlast"),
        "# A\n\ntext\n\n- x\n\n> q\n\nlast\n"
    );
    assert_eq!(round_trip(""), "");
}
