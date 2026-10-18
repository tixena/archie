//! The tree written to JSON and read back, and the TypeScript its types
//! publish. The JSON holds the markdown's own tree: no node writes its
//! `kind`, and `rebuild_with` builds it again.

use super::blocks::{nested_grid_tables, nested_markdown_blocks};
use super::extension::Comments;
use super::on_two_megabytes;
use crate::parser::ast::{
    Args, Block, BlockQuote, Code, CodeBlock, Document, Emphasis, Frontmatter, FrontmatterFormat,
    Heading, Html, Image, Inline, Item, Kind, Line, LineBreak, Link, List, MarkdownBlock,
    NonFinite, Paragraph, Section, Strong, Table, TableCell, TableRow, Text, ThematicBreak, Value,
};
use crate::parser::error::RebuildError;
use crate::parser::extension::{Plain, With};
use crate::parser::inline::INLINE_CAP;
use crate::parser::lexer::BLOCK_CAP;
use crate::parser::{ParseOptions, from_json, parse, parse_with, rebuild_with, to_json};
use crate::writer::{WriteOptions, write};

/// Parses `source`, writes the tree to JSON and reads it back: the same
/// tree, every `kind` empty.
pub(super) fn reads_back_from_json(source: &str) {
    let mut parsed = parse(source);
    let back: Document = from_json(&to_json(&parsed).unwrap()).unwrap();
    unclaim(&mut parsed);
    assert_eq!(back, parsed);
}

/// Parses `source` with `Comments`, writes the tree to JSON, reads it back
/// and builds its extension nodes again: the tree that `source` written in
/// archie's form, its frontmatter in its own syntax, parses into.
pub(super) fn rebuilds_from_json(source: &str) {
    let options = ParseOptions::default();
    let parsed = parse_with::<Comments>(source, &options).unwrap();
    let back: Document<With<Comments>> = from_json(&to_json(&parsed).unwrap()).unwrap();
    let own = WriteOptions {
        frontmatter: parsed.frontmatter.as_ref().map_or_else(
            FrontmatterFormat::default,
            |frontmatter| match frontmatter {
                Ok(read) => read.format,
                Err(broken) => broken.format,
            },
        ),
    };
    let written = parse_with::<Comments>(&write(&parsed, &own).unwrap(), &options).unwrap();
    assert_eq!(rebuild_with::<Comments>(&back, &options).unwrap(), written);
}

#[test]
fn the_sample_document_reads_back_from_json() {
    let document = parse("---\ntitle: Notes\n---\n# A\ntext\n");
    let json = to_json(&document).unwrap();
    assert_eq!(
        json,
        concat!(
            r#"{"body":{"sections":[{"blocks":[{"Paragraph":{"lines":[{"children":[{"Text":"#,
            r#"{"span":{"start":25,"end":29},"value":"text"}}],"span":{"start":25,"end":29}}],"#,
            r#""span":{"start":25,"end":29}}}],"heading":{"children":[{"Text":"#,
            r#"{"span":{"start":23,"end":24},"value":"A"}}],"span":{"start":21,"end":24}},"#,
            r#""span":{"start":21,"end":29},"subsections":[]}],"span":{"start":21,"end":30}},"#,
            r#""frontmatter":{"Ok":{"format":"Yaml","mapping":{"title":{"String":"Notes"}},"#,
            r#""span":{"start":0,"end":20}}},"span":{"start":0,"end":30}}"#,
        )
    );
    let back: Document = from_json(&json).unwrap();
    assert_eq!(back, document);
}

#[test]
fn the_entries_a_frontmatter_drops_are_written_to_json_and_read_back() {
    let document = parse("---\ntitle: Notes\ntags: [a, b\ndate: 2026-10-17\n---\n# Body\n");
    let json = to_json(&document).unwrap();
    assert!(
        json.contains(concat!(
            r#""dropped":[{"message":"a line of a flow collection must be indented "#,
            r#"more than its key","span":{"start":17,"end":28}}]"#,
        )),
        "{json}"
    );
    let back: Document = from_json(&json).unwrap();
    assert_eq!(back, document);
    let definition = Frontmatter::ts_definition();
    assert!(
        definition.contains("dropped?: Array<ParseError>;"),
        "{definition}"
    );
}

#[test]
fn a_broken_frontmatter_is_written_to_json_as_err_and_read_back() {
    let source = "---\n{a: 1, b: [}\n---\n# A\n";
    let document = parse(source);
    let json = to_json(&document).unwrap();
    assert!(
        json.contains(concat!(
            r#""frontmatter":{"Err":{"error":{"message":"expected `,` or `]` in the list","#,
            r#""span":{"start":15,"end":15}},"format":"Yaml","span":{"start":0,"end":20},"#,
            r#""text":"{a: 1, b: [}"}}"#,
        )),
        "{json}"
    );
    let back: Document = from_json(&json).unwrap();
    assert_eq!(back, document);
    reads_back_from_json(source);
    rebuilds_from_json(source);
    let definition = <Document>::ts_definition();
    assert!(
        definition.contains("{ Ok: Frontmatter } | { Err: BrokenFrontmatter }"),
        "{definition}"
    );
}

#[test]
fn a_tree_at_the_caps_reads_back_from_json() {
    on_two_megabytes(|| {
        let stars = "*".repeat(2 * INLINE_CAP);
        for source in [
            format!("{stars}a{stars}\n"),
            format!("{}a\n", "- ".repeat(BLOCK_CAP)),
            nested_grid_tables(BLOCK_CAP),
        ] {
            reads_back_from_json(&source);
        }
        let options = ParseOptions::default();
        let parsed = parse_with::<Plain>(&nested_markdown_blocks(BLOCK_CAP), &options).unwrap();
        let back: Document<With<Plain>> = from_json(&to_json(&parsed).unwrap()).unwrap();
        assert_eq!(rebuild_with::<Plain>(&back, &options).unwrap(), parsed);
    });
}

#[test]
fn json_that_is_not_a_document_does_not_read() {
    let read: Result<Document, _> = from_json("{}");
    let error = read.unwrap_err().to_string();
    assert!(error.starts_with("missing field `body`"), "{error}");
}

#[test]
fn a_span_is_written_as_its_start_and_its_end() {
    let json = to_json(&parse("  text\n")).unwrap();
    assert!(
        json.contains(r#"{"Text":{"span":{"start":2,"end":6},"value":"text"}}"#),
        "{json}"
    );
}

#[test]
fn the_claim_pass_builds_a_claimed_block_again_from_json() {
    let source = "```markdown\n# Inner\n```\n";
    let options = ParseOptions::default();
    let parsed = parse_with::<Plain>(source, &options).unwrap();
    let back: Document<With<Plain>> = from_json(&to_json(&parsed).unwrap()).unwrap();
    let claimed = |document: &Document<With<Plain>>| {
        matches!(
            &document.body.sections[0].blocks[0],
            Block::CodeBlock(block) if block.kind.0.is_some()
        )
    };
    assert!(claimed(&parsed));
    assert!(!claimed(&back));
    assert_eq!(rebuild_with::<Plain>(&back, &options).unwrap(), parsed);
}

#[test]
fn a_toml_frontmatter_holding_a_null_is_rebuilt_in_yaml() {
    let options = ParseOptions::default();
    let source = "+++\ntitle = \"Notes\"\n+++\n```markdown\n# A\n```\n";
    let mut parsed = parse_with::<Plain>(source, &options).unwrap();
    let frontmatter = parsed.frontmatter.as_mut().unwrap().as_mut().unwrap();
    frontmatter.mapping["title"] = Value::Null;
    let back: Document<With<Plain>> = from_json(&to_json(&parsed).unwrap()).unwrap();
    let rebuilt = rebuild_with::<Plain>(&back, &options).unwrap();
    let yaml = WriteOptions {
        frontmatter: FrontmatterFormat::Yaml,
    };
    let written = parse_with::<Plain>(&write(&parsed, &yaml).unwrap(), &options).unwrap();
    assert_eq!(
        rebuilt
            .frontmatter
            .as_ref()
            .map(|read| read.as_ref().unwrap().format),
        Some(FrontmatterFormat::Yaml)
    );
    assert_eq!(rebuilt, written);
}

#[test]
fn a_null_in_the_toml_args_of_a_code_block_fails_the_rebuild_naming_the_key() {
    let options = ParseOptions::default();
    let mut parsed = parse_with::<Plain>("```sh a = 1, b = 2\n```\n", &options).unwrap();
    if let Block::CodeBlock(code) = &mut parsed.body.sections[0].blocks[0]
        && let Args::Toml(mapping) = &mut code.args
    {
        mapping["a"] = Value::Null;
    }
    let back: Document<With<Plain>> = from_json(&to_json(&parsed).unwrap()).unwrap();
    let rebuilt = rebuild_with::<Plain>(&back, &options);
    assert!(
        matches!(&rebuilt, Err(RebuildError::Write(error)) if error.key == "a"),
        "{rebuilt:?}"
    );
}

#[test]
fn a_non_finite_float_reads_back_from_json() {
    for (source, value) in [
        ("---\nx: .nan\n---\n", NonFinite::Nan),
        ("---\nx: .inf\n---\n", NonFinite::Infinity),
        ("---\nx: -.inf\n---\n", NonFinite::NegativeInfinity),
        ("+++\nx = nan\n+++\n", NonFinite::Nan),
    ] {
        let document = parse(source);
        let back: Document = from_json(&to_json(&document).unwrap()).unwrap();
        assert_eq!(back, document);
        assert_eq!(scalar(&back, "x"), Value::NonFinite(value));
    }
}

#[test]
fn a_non_finite_float_is_written_as_its_name() {
    let json = to_json(&parse("---\nx: .nan\n---\n")).unwrap();
    assert!(json.contains(r#""x":{"NonFinite":"Nan"}"#), "{json}");
}

#[test]
fn the_typescript_of_a_value_names_the_non_finite_floats() {
    let value = Value::ts_definition();
    let non_finite = NonFinite::ts_definition();
    assert!(value.contains(r#""NonFinite": NonFinite;"#), "{value}");
    assert!(
        non_finite.contains(
            "export type NonFinite =\n  | \"Infinity\"\n  | \"Nan\"\n  | \"NegativeInfinity\";"
        ),
        "{non_finite}"
    );
}

#[test]
fn the_typescript_of_a_document_names_its_nodes() {
    let document = <Document>::ts_definition();
    let block = <MarkdownBlock>::ts_definition();
    eprintln!("{document}\n{block}");
    assert!(document.contains("body: MarkdownBlock<E>;"), "{document}");
    assert!(
        document.contains("span: { start: number; end: number };"),
        "{document}"
    );
    assert!(block.contains("sections: Array<Section<E>>;"), "{block}");
}

#[test]
fn every_span_is_a_start_and_an_end_and_no_node_writes_its_kind() {
    let definitions = [
        <Document>::ts_definition(),
        Frontmatter::ts_definition(),
        <MarkdownBlock>::ts_definition(),
        <BlockQuote>::ts_definition(),
        <Html>::ts_definition(),
        <ThematicBreak>::ts_definition(),
        <Section>::ts_definition(),
        <Heading>::ts_definition(),
        <Paragraph>::ts_definition(),
        <Line>::ts_definition(),
        <List>::ts_definition(),
        <Item>::ts_definition(),
        <CodeBlock>::ts_definition(),
        <Table>::ts_definition(),
        <TableRow>::ts_definition(),
        <TableCell>::ts_definition(),
        <Text>::ts_definition(),
        <Emphasis>::ts_definition(),
        <Strong>::ts_definition(),
        <LineBreak>::ts_definition(),
        <Link>::ts_definition(),
        <Image>::ts_definition(),
        <Code>::ts_definition(),
    ];
    for definition in definitions {
        assert!(
            definition.contains("  span: { start: number; end: number };"),
            "{definition}"
        );
        assert!(!definition.contains("kind"), "{definition}");
    }
    let value = Value::ts_definition();
    assert!(
        !value.contains("span") && !value.contains("kind"),
        "{value}"
    );
}

/// The value under `key` in the frontmatter of `document`.
fn scalar(document: &Document, key: &str) -> Value {
    let mapping = &document
        .frontmatter
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .mapping;
    mapping[key].clone()
}

/// Empties the `kind` of every node of `document`.
fn unclaim(document: &mut Document) {
    document.kind = Kind(None);
    unclaim_markdown_block(&mut document.body);
}

fn unclaim_markdown_block(block: &mut MarkdownBlock) {
    block.kind = Kind(None);
    block.sections.iter_mut().for_each(unclaim_section);
}

fn unclaim_section(section: &mut Section) {
    section.kind = Kind(None);
    if let Some(heading) = &mut section.heading {
        heading.kind = Kind(None);
        unclaim_inlines(&mut heading.children);
    }
    for block in &mut section.blocks {
        match block {
            Block::BlockQuote(block_quote) => block_quote.kind = Kind(None),
            Block::CodeBlock(code_block) => code_block.kind = Kind(None),
            Block::Html(html) => html.kind = Kind(None),
            Block::List(list) => {
                list.kind = Kind(None);
                for item in &mut list.items {
                    item.kind = Kind(None);
                    unclaim_markdown_block(&mut item.body);
                }
            }
            Block::Paragraph(paragraph) => {
                paragraph.kind = Kind(None);
                for line in &mut paragraph.lines {
                    line.kind = Kind(None);
                    unclaim_inlines(&mut line.children);
                }
            }
            Block::Table(table) => {
                table.kind = Kind(None);
                for row in table.head.iter_mut().chain(&mut table.rows) {
                    row.kind = Kind(None);
                    for cell in &mut row.cells {
                        cell.kind = Kind(None);
                        unclaim_markdown_block(&mut cell.body);
                    }
                }
            }
            Block::ThematicBreak(thematic_break) => thematic_break.kind = Kind(None),
        }
    }
    section.subsections.iter_mut().for_each(unclaim_section);
}

fn unclaim_inlines(inlines: &mut [Inline]) {
    for inline in inlines {
        match inline {
            Inline::Code(code) => code.kind = Kind(None),
            Inline::Emphasis(emphasis) => {
                emphasis.kind = Kind(None);
                unclaim_inlines(&mut emphasis.children);
            }
            Inline::Html(html) => html.kind = Kind(None),
            Inline::Image(image) => image.kind = Kind(None),
            Inline::LineBreak(line_break) => line_break.kind = Kind(None),
            Inline::Link(link) => {
                link.kind = Kind(None);
                unclaim_inlines(&mut link.children);
            }
            Inline::Strong(strong) => {
                strong.kind = Kind(None);
                unclaim_inlines(&mut strong.children);
            }
            Inline::Text(text) => text.kind = Kind(None),
        }
    }
}
