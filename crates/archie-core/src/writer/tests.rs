//! Each sample is a tree written back to markdown: read from a source, or
//! built, then written, then read again.

use indexmap::IndexMap;

use crate::parser::ast::{
    Block, Document, Frontmatter, FrontmatterFormat, Heading, Inline, Kind, Line, Mapping,
    MarkdownBlock, Pair, Paragraph, Scalar, ScalarValue, Section, Sequence, Table, Text, Value,
};
use crate::parser::extension::Markdown;
use crate::parser::parse;
use crate::writer::{WriteError, WriteOptions, write};

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

/// The owner's sample of archie's written form.
const OWNERS_SAMPLE: &str = "---
title: 'Notes'
tags: [a, b]
---
Notes
=====

Some __bold__ and _em_ text.

* one
*   two
    continued

~~~python
print(1)
~~~

___
";

/// TOML frontmatter with a table, then a heading.
const TOML_SAMPLE: &str =
    "+++\ntitle = \"Notes\"\ndue = 2026-10-10\n[params]\nauthor = \"Ada\"\n+++\n# Notes\n";

/// Frontmatter written in TOML.
const TOML: WriteOptions = WriteOptions {
    frontmatter: FrontmatterFormat::Toml,
};

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
        // A time of day reads back from YAML as text, which needs no quotes.
        let again = text.replace("standup_at: \"09:30:00\"", "standup_at: 09:30:00");
        assert_eq!(written(&text), again, "{name}");
    }
}

#[test]
fn the_owners_sample_is_written_in_archies_form() {
    let source = "\
Notes
=====

Some __bold__ and _em_ text.

* one
*   two
    continued

~~~python
print(1)
~~~

___
";
    assert_eq!(
        round_trip(source),
        "\
# Notes

Some **bold** and *em* text.

- one
- two
  continued

```python
print(1)
```

---
"
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
fn a_text_line_that_would_open_a_tilde_fence_is_escaped() {
    assert_eq!(round_trip("\\~~~\n"), "\\~~~\n");
    assert_eq!(round_trip("a\n\\~~~ b\n"), "a\n\\~~~ b\n");
    assert_eq!(round_trip("~~\n"), "~~\n");
}

#[test]
fn a_first_body_line_that_would_open_a_toml_frontmatter_is_escaped() {
    let text = round_trip("\\+++\nx = 1\n\\+++\n");
    assert_eq!(text, "\\+++\nx = 1\n+++\n");
    assert!(parse(&text).unwrap().frontmatter.is_none());
    assert_eq!(round_trip("+++ x\n"), "+++ x\n");
    assert_eq!(round_trip("\\---\nx: 1\n\\---\n"), "\\---\nx: 1\n\\---\n");
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

fn toml(source: &str) -> String {
    write(&parse(source).unwrap(), &TOML).unwrap()
}

fn toml_error(source: &str) -> WriteError {
    write(&parse(source).unwrap(), &TOML).unwrap_err()
}

/// The frontmatter mapping `source` reads as, every span emptied.
fn mapping(source: &str) -> Mapping {
    let mut read = parse(source).unwrap().frontmatter.unwrap().mapping;
    clear_mapping(&mut read);
    read
}

fn clear_mapping(mapping: &mut Mapping) {
    mapping.span = 0..0;
    for pair in mapping.pairs.values_mut() {
        pair.repeats.clear();
        pair.span = 0..0;
        clear_value(&mut pair.value);
    }
}

fn clear_value(value: &mut Value) {
    match value {
        Value::Mapping(mapping) => clear_mapping(mapping),
        Value::Scalar(scalar) => scalar.span = 0..0,
        Value::Sequence(sequence) => {
            sequence.span = 0..0;
            for item in &mut sequence.items {
                clear_value(item);
            }
        }
    }
}

/// `mapping` with each time of day turned into the text it is written as
/// in YAML.
fn times_as_text(mapping: &mut Mapping) {
    for pair in mapping.pairs.values_mut() {
        time_as_text(&mut pair.value);
    }
}

fn time_as_text(value: &mut Value) {
    match value {
        Value::Mapping(mapping) => times_as_text(mapping),
        Value::Scalar(scalar) => {
            if let ScalarValue::Time(time) = scalar.value {
                scalar.value = ScalarValue::String(time.to_string());
            }
        }
        Value::Sequence(sequence) => {
            for item in &mut sequence.items {
                time_as_text(item);
            }
        }
    }
}

fn null_error(key: &str) -> WriteError {
    WriteError {
        key: key.to_owned(),
        message: "TOML cannot hold a null".to_owned(),
    }
}

fn text(value: &str) -> Value {
    Value::Scalar(Scalar {
        span: 0..0,
        value: ScalarValue::String(value.to_owned()),
    })
}

/// A document whose frontmatter holds `pairs` and whose body is empty.
fn with_frontmatter(pairs: IndexMap<String, Pair>) -> Document {
    Document {
        frontmatter: Some(Frontmatter {
            format: FrontmatterFormat::Yaml,
            mapping: Mapping { pairs, span: 0..0 },
            span: 0..0,
        }),
        ..document(Vec::new())
    }
}

#[test]
fn toml_frontmatter_is_written_as_yaml_by_default() {
    assert_eq!(
        written(TOML_SAMPLE),
        "---\ntitle: Notes\ndue: !!timestamp 2026-10-10\nparams:\n  author: Ada\n---\n# Notes\n"
    );
}

#[test]
fn toml_frontmatter_is_written_between_plus_lines_when_picked() {
    assert_eq!(
        toml(TOML_SAMPLE),
        "+++\ntitle = \"Notes\"\ndue = 2026-10-10\n\n[params]\nauthor = \"Ada\"\n+++\n# Notes\n"
    );
}

#[test]
fn keys_keep_the_order_they_were_read_and_a_key_read_twice_is_written_once() {
    let source = "---\nz: 1\nt: A\na: 2\nt: B\n---\n";
    assert_eq!(written(source), "---\nz: 1\nt: [A, B]\na: 2\n---\n");
    assert_eq!(toml(source), "+++\nz = 1\nt = [\"A\", \"B\"]\na = 2\n+++\n");
}

#[test]
fn yaml_is_written_in_block_form_with_text_quoted_only_when_needed() {
    let source = "---
title: 'Notes'
owner:
  name: Ada
  team:
    lead: Grace
tags: [a, b]
people:
  - name: Ada
    role: host
  - Grace
nested: [[a], []]
none: {}
empty: ''
a: \"yes\"
b: \"2026-10-10\"
c: \"12\"
pair: \"a: b\"
comment: \"a #b\"
dash: \"- a\"
lines: \"one\\ntwo\"
nothing: null
due: 2026-10-10
at: 2026-10-10T07:32:00Z
local: 2026-10-10T07:32:00.5
---
";
    let text = written(source);
    assert_eq!(
        text,
        "---
title: Notes
owner:
  name: Ada
  team:
    lead: Grace
tags: [a, b]
people:
  - name: Ada
    role: host
  - Grace
nested: [[a], []]
none: {}
empty: \"\"
a: yes
b: \"2026-10-10\"
c: \"12\"
pair: \"a: b\"
comment: \"a #b\"
dash: \"- a\"
lines: \"one\\ntwo\"
nothing: null
due: !!timestamp 2026-10-10
at: !!timestamp 2026-10-10T07:32:00Z
local: !!timestamp 2026-10-10T07:32:00.500
---
"
    );
    assert_eq!(mapping(&text), mapping(source));
}

#[test]
fn a_list_of_mappings_is_written_in_block_form_inside_a_list_too() {
    assert_eq!(
        written("---\np:\n  - name: Ada\n---\n"),
        "---\np:\n  - name: Ada\n---\n"
    );
    let source = "---\np: [[{name: Ada}, b], c]\n---\n";
    let text = written(source);
    assert_eq!(text, "---\np:\n  - - name: Ada\n    - b\n  - c\n---\n");
    assert_eq!(mapping(&text), mapping(source));
}

#[test]
fn a_time_of_day_is_written_into_yaml_as_quoted_text() {
    assert_eq!(
        written("+++\nat = 07:32:00\n+++\n"),
        "---\nat: \"07:32:00\"\n---\n"
    );
}

#[test]
fn toml_quotes_all_text_and_puts_tables_after_the_plain_keys() {
    let source = "---
title: Notes
owner: [Ada, {name: Grace}]
params:
  author: Ada
  extra:
    toc: true
  tags: [a, b]
count: 3
people:
  - name: Ada
  - name: Grace
ratio: 0.5
whole: 2.0
done: false
due: 2026-10-10
at: 2026-10-10T07:32:00-04:00
local: 2026-10-10T07:32:00
none: {}
quote: 'say \"hi\" \\ now'
lines: \"one\\ntwo\"
---
";
    let text = toml(source);
    assert_eq!(
        text,
        "+++
title = \"Notes\"
owner = [\"Ada\", { name = \"Grace\" }]
count = 3
ratio = 0.5
whole = 2.0
done = false
due = 2026-10-10
at = 2026-10-10T07:32:00-04:00
local = 2026-10-10T07:32:00
quote = \"say \\\"hi\\\" \\\\ now\"
lines = \"one\\ntwo\"

[params]
author = \"Ada\"
tags = [\"a\", \"b\"]

[params.extra]
toc = true

[[people]]
name = \"Ada\"

[[people]]
name = \"Grace\"

[none]
+++
"
    );
    assert_eq!(mapping(&text), mapping(source));
    assert_eq!(
        toml("+++\nat = 07:32:00\n+++\n"),
        "+++\nat = 07:32:00\n+++\n"
    );
}

#[test]
fn a_null_cannot_be_written_into_toml() {
    assert_eq!(toml_error("---\na: null\n---\n"), null_error("a"));
    assert_eq!(
        toml_error("---\nowner:\n  name: Ada\n  email: null\n---\n"),
        null_error("owner.email")
    );
    assert_eq!(
        toml_error("---\nowner: [Ada, {email: ~}]\n---\n"),
        null_error("owner.email")
    );
    assert_eq!(
        toml_error("---\np:\n  - name:\n---\n"),
        null_error("p.name")
    );
    assert_eq!(toml_error("---\na.b: null\n---\n"), null_error("\"a.b\""));
}

#[test]
fn every_fixture_frontmatter_reads_back_to_the_same_mapping_in_yaml_and_in_toml() {
    let mut with_frontmatter_count = 0_usize;
    for (name, source) in FIXTURES {
        if parse(source).unwrap().frontmatter.is_none() {
            continue;
        }
        with_frontmatter_count += 1;
        let mut expected = mapping(source);
        match name {
            "meeting_notes" => assert_eq!(toml_error(source), null_error("recording")),
            "recipe" => assert_eq!(toml_error(source), null_error("source")),
            _ => assert_eq!(mapping(&toml(source)), expected, "{name}"),
        }
        times_as_text(&mut expected);
        assert_eq!(mapping(&written(source)), expected, "{name}");
    }
    assert_eq!(with_frontmatter_count, 15);
}

#[test]
fn a_body_line_after_the_frontmatter_is_not_escaped() {
    let source = "---\na: 1\n---\n\\+++\n";
    assert_eq!(round_trip(source), "---\na: 1\n---\n+++\n");
    let text = toml(source);
    assert_eq!(text, "+++\na = 1\n+++\n+++\n");
    assert_eq!(mapping(&text), mapping(source));
    assert_eq!(body(&text), body(source));
}

#[test]
fn the_owners_sample_frontmatter_is_written_with_its_quotes_dropped() {
    let text = written(OWNERS_SAMPLE);
    assert!(
        text.starts_with("---\ntitle: Notes\ntags: [a, b]\n---\n# Notes\n"),
        "{text}"
    );
}

#[test]
fn text_reads_back_as_the_same_text_whether_it_is_quoted_or_not() {
    let texts = [
        "",
        " a",
        "a ",
        "a: b",
        "a:",
        "a:b",
        "a #b",
        "a#b",
        "#a",
        "- a",
        "-a",
        "? a",
        ":a",
        "[a]",
        "{a}",
        "a,b",
        "a]b",
        "'a'",
        "\"a\"",
        "!a",
        "&a",
        "*a",
        "|a",
        ">a",
        "%a",
        "@a",
        "`a`",
        "one\ntwo",
        "tab\there",
        "null",
        "~",
        "true",
        "False",
        "12",
        "-5",
        "1.5",
        "1e5",
        ".inf",
        "0x1F",
        "2026-10-10",
        "2026-02-30",
        "2026-10-10T07:32:00Z",
        "yes",
        "07:32:00",
        "a b",
        "back\\slash",
        "say \"hi\"",
        "https://x.org/a#b",
        "\u{1}",
        "\u{7f}",
        "\u{85}",
        "\u{e9}t\u{e9}",
        "---",
        "...",
        "+++",
    ];
    let mut pairs: IndexMap<String, Pair> = texts
        .iter()
        .map(|value| {
            let pair = Pair {
                repeats: Vec::new(),
                span: 0..0,
                value: text(value),
            };
            ((*value).to_owned(), pair)
        })
        .collect();
    let list = Value::Sequence(Sequence {
        items: texts.iter().map(|value| text(value)).collect(),
        span: 0..0,
    });
    pairs.insert(
        "list".to_owned(),
        Pair {
            repeats: Vec::new(),
            span: 0..0,
            value: list,
        },
    );
    let built = with_frontmatter(pairs);
    let mut expected = built.frontmatter.clone().unwrap().mapping;
    clear_mapping(&mut expected);
    for frontmatter in [FrontmatterFormat::Toml, FrontmatterFormat::Yaml] {
        let written_text = write(&built, &WriteOptions { frontmatter }).unwrap();
        assert_eq!(mapping(&written_text), expected, "{written_text}");
    }
}
