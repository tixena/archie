//! Extensions declared outside archie's tree: a comment block shaped like
//! remargin's, read into a typed node, and a whole file read as a ticket.

use core::ops::Range;

use super::{Rows, on_two_megabytes, outline};
use crate::parser::ast::{
    Block, CodeBlock, Document, Inline, Mapping, MarkdownBlock, ScalarValue, TableCell, Value,
};
use crate::parser::error::ParseError;
use crate::parser::extension::{Construct, Context, Either, Extension, Plain, With};
use crate::parser::lexer::BLOCK_CAP;
use crate::parser::{
    ParseOptions, from_json, parse, parse_with, read_toml_lines, read_yaml_lines, rebuild_with,
    to_json,
};
use crate::writer::{WriteOptions, Writer, write};

const SOURCE: &str = "\
Intro.

```remargin
---
id: abc
author: ada
type: human
to: [grace]
reactions:
  thumbs:
    - author: grace
      ts: 2026-10-01T10:00:00Z
ack:
  - grace@2026-10-01T10:00:00Z
---
Looks good.
```

```rust
fn main() {}
```
";

const NO_HEADER: &str = "a comment opens with a header between `---` or `+++` lines";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Comments;

/// A comment: its YAML or TOML header, and its body read as markdown with the
/// extensions the file is parsed with.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Comment<T>
where
    T: Extension,
{
    body: MarkdownBlock<T>,
    header: Mapping,
}

impl Extension for Comments {
    type Node<T>
        = Comment<T>
    where
        T: Extension;

    fn claim<T>(
        construct: Construct<'_, T>,
        context: &Context<'_>,
    ) -> Option<Result<Comment<T>, ParseError>>
    where
        T: Extension,
    {
        let Construct::CodeBlock(block) = construct else {
            return None;
        };
        (block.language.as_deref() == Some("remargin")).then(|| comment(block, context))
    }

    /// The header's text values, a `key: value` line each, between `---`
    /// lines, then the body.
    fn write<T>(node: &Comment<T>, _plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension,
    {
        writer.fenced("remargin", |inner| {
            inner.push_str("---\n");
            for (key, pair) in &node.header.pairs {
                if let Value::Scalar(scalar) = &pair.value
                    && let ScalarValue::String(text) = &scalar.value
                {
                    inner.push_str(key);
                    inner.push_str(": ");
                    inner.push_str(text);
                    inner.push_str("\n");
                }
            }
            inner.push_str("---\n");
            inner.markdown_block(&node.body);
        });
    }
}

/// Reads a file with a title in its frontmatter as a ticket.
#[derive(Debug, Clone, PartialEq)]
struct Tickets;

/// A ticket: its title, and how many of its top-level code blocks another
/// extension claimed before the document was offered.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ticket {
    claimed: usize,
    title: String,
}

impl Extension for Tickets {
    type Node<T>
        = Ticket
    where
        T: Extension;

    fn claim<T>(
        construct: Construct<'_, T>,
        _context: &Context<'_>,
    ) -> Option<Result<Ticket, ParseError>>
    where
        T: Extension,
    {
        let Construct::Document(document) = construct else {
            return None;
        };
        let read = document.frontmatter.as_ref()?.as_ref().ok()?;
        let pair = read.mapping.pairs.get("title")?;
        let Value::Scalar(scalar) = &pair.value else {
            return None;
        };
        let ScalarValue::String(title) = &scalar.value else {
            return None;
        };
        let claimed = code_blocks(&document.body)
            .into_iter()
            .filter(|block| block.kind.0.is_some())
            .count();
        Some(Ok(Ticket {
            claimed,
            title: title.clone(),
        }))
    }

    fn write<T>(_node: &Self::Node<T>, plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension,
    {
        writer.construct(plain);
    }
}

/// Claims every code block, as its info string as the writer writes it.
#[derive(Debug, Clone, PartialEq)]
struct Fences;

impl Extension for Fences {
    type Node<T>
        = String
    where
        T: Extension;

    fn claim<T>(
        construct: Construct<'_, T>,
        _context: &Context<'_>,
    ) -> Option<Result<String, ParseError>>
    where
        T: Extension,
    {
        let Construct::CodeBlock(block) = construct else {
            return None;
        };
        Some(Ok(Writer::new().info_string(block)))
    }

    fn write<T>(_node: &Self::Node<T>, plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension,
    {
        writer.construct(plain);
    }
}

/// Claims every table cell, as the markdown block its span reads as.
#[derive(Debug, Clone, PartialEq)]
struct CellText;

impl Extension for CellText {
    type Node<T>
        = MarkdownBlock<T>
    where
        T: Extension;

    fn claim<T>(
        construct: Construct<'_, T>,
        context: &Context<'_>,
    ) -> Option<Result<MarkdownBlock<T>, ParseError>>
    where
        T: Extension,
    {
        let Construct::TableCell(cell) = construct else {
            return None;
        };
        Some(context.markdown::<T>(cell.span.clone()))
    }

    fn write<T>(_node: &Self::Node<T>, plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension,
    {
        writer.construct(plain);
    }
}

/// Claims every block quote, HTML block and thematic break, as a label.
#[derive(Debug, Clone, PartialEq)]
struct Raw;

impl Extension for Raw {
    type Node<T>
        = String
    where
        T: Extension;

    fn claim<T>(
        construct: Construct<'_, T>,
        _context: &Context<'_>,
    ) -> Option<Result<String, ParseError>>
    where
        T: Extension,
    {
        if let Construct::BlockQuote(quote) = construct {
            return Some(Ok(format!("quote {}", quote.value)));
        }
        if let Construct::Html(html) = construct {
            return Some(Ok(format!("html {}", html.value)));
        }
        matches!(construct, Construct::ThematicBreak(_)).then(|| Ok("break".to_owned()))
    }

    fn write<T>(_node: &Self::Node<T>, plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension,
    {
        writer.construct(plain);
    }
}

/// Claims every emphasis, strong emphasis, inline HTML and hard line break,
/// as a label.
#[derive(Debug, Clone, PartialEq)]
struct Marks;

impl Extension for Marks {
    type Node<T>
        = String
    where
        T: Extension;

    fn claim<T>(
        construct: Construct<'_, T>,
        _context: &Context<'_>,
    ) -> Option<Result<String, ParseError>>
    where
        T: Extension,
    {
        if let Construct::Html(html) = construct {
            return Some(Ok(format!("html {}", html.value)));
        }
        if matches!(construct, Construct::Emphasis(_)) {
            return Some(Ok("emphasis".to_owned()));
        }
        if matches!(construct, Construct::Strong(_)) {
            return Some(Ok("strong".to_owned()));
        }
        matches!(construct, Construct::LineBreak(_)).then(|| Ok("break".to_owned()))
    }

    fn write<T>(_node: &Self::Node<T>, plain: Construct<'_, T>, writer: &mut Writer)
    where
        T: Extension,
    {
        writer.construct(plain);
    }
}

/// A `---` line, the YAML header, a `---` line, then the body; or the same
/// with a TOML header between `+++` lines.
fn comment<T>(block: &CodeBlock<T>, context: &Context<'_>) -> Result<Comment<T>, ParseError>
where
    T: Extension,
{
    let refused = || ParseError::new(NO_HEADER, block.content.clone());
    let lines = content_lines(&block.content, context);
    let text = |line: &Range<usize>| context.source[line.clone()].trim();
    let fence = lines
        .first()
        .map(text)
        .filter(|first| matches!(*first, "---" | "+++"))
        .ok_or_else(refused)?;
    let close = lines
        .iter()
        .skip(1)
        .position(|line| text(line) == fence)
        .ok_or_else(refused)?
        + 1;
    let header = &lines[1..close];
    let header_range = header.first().zip(header.last()).map_or_else(
        || lines[close].start..lines[close].start,
        |(first, last)| first.start..last.end,
    );
    let read = if fence == "+++" {
        read_toml_lines
    } else {
        read_yaml_lines
    };
    Ok(Comment {
        body: context.markdown::<T>(lines[close].end..block.content.end)?,
        header: read(context.source, &context.lines_of(header_range))?,
    })
}

/// The lines of `range`: in a grid cell, its parts on the cell's lines;
/// elsewhere, each line of the source it covers, with its line ending.
fn content_lines(range: &Range<usize>, context: &Context<'_>) -> Vec<Range<usize>> {
    if let Some(cell) = context.lines {
        return cell
            .iter()
            .filter(|line| line.start <= range.end && range.start <= line.end)
            .map(|line| line.start.max(range.start)..line.end.min(range.end))
            .collect();
    }
    let mut lines = Vec::new();
    let mut at = range.start;
    for line in context.source[range.clone()].split_inclusive('\n') {
        lines.push(at..at + line.len());
        at += line.len();
    }
    lines
}

/// The code blocks among the blocks of the top-level sections of
/// `markdown`.
fn code_blocks<T>(markdown: &MarkdownBlock<T>) -> Vec<&CodeBlock<T>>
where
    T: Extension,
{
    markdown
        .sections
        .iter()
        .flat_map(|section| &section.blocks)
        .filter_map(|block| match block {
            Block::CodeBlock(code_block) => Some(code_block),
            Block::BlockQuote(_)
            | Block::Html(_)
            | Block::List(_)
            | Block::Paragraph(_)
            | Block::Table(_)
            | Block::ThematicBreak(_) => None,
        })
        .collect()
}

/// The node of the first extension of a pair, when it claimed the construct.
const fn first<A, B>(node: &Either<A, B>) -> Option<&A> {
    match node {
        Either::First(first) => Some(first),
        Either::Second(_) => None,
    }
}

/// The node of the second extension of a pair, when it claimed the
/// construct.
const fn second<A, B>(node: &Either<A, B>) -> Option<&B> {
    match node {
        Either::First(_) => None,
        Either::Second(second) => Some(second),
    }
}

/// The body of the first item of the first list among the blocks of the
/// first section of `markdown`.
fn first_item<T>(markdown: &MarkdownBlock<T>) -> &MarkdownBlock<T>
where
    T: Extension,
{
    markdown.sections[0]
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::List(list) => list.items.first().map(|item| &item.body),
            Block::BlockQuote(_)
            | Block::CodeBlock(_)
            | Block::Html(_)
            | Block::Paragraph(_)
            | Block::Table(_)
            | Block::ThematicBreak(_) => None,
        })
        .unwrap()
}

/// The first cell of the first table among the blocks of the first section
/// of `markdown`.
fn first_cell<T>(markdown: &MarkdownBlock<T>) -> &TableCell<T>
where
    T: Extension,
{
    markdown.sections[0]
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Table(table) => table.head.iter().chain(&table.rows).next()?.cells.first(),
            Block::BlockQuote(_)
            | Block::CodeBlock(_)
            | Block::Html(_)
            | Block::List(_)
            | Block::Paragraph(_)
            | Block::ThematicBreak(_) => None,
        })
        .unwrap()
}

/// A grid table of one row: `text` in its first cell, and `x` on each line
/// of its second.
fn in_grid_cell(text: &str) -> String {
    let width = text.lines().map(str::len).max().unwrap_or_default();
    let border = format!("+{}+---+\n", "-".repeat(width + 2));
    let mut grid = border.clone();
    for line in text.lines() {
        grid.push_str("| ");
        grid.push_str(line);
        grid.push_str(&" ".repeat(width - line.len()));
        grid.push_str(" | x |\n");
    }
    grid.push_str(&border);
    grid
}

/// `text` with every line that holds text indented by `indent`.
fn indented(text: &str, indent: &str) -> String {
    let mut lines = String::new();
    for line in text.lines() {
        if !line.is_empty() {
            lines.push_str(indent);
        }
        lines.push_str(line);
        lines.push('\n');
    }
    lines
}

/// Each node of `block`, with the source text its span covers.
fn block_spans<'source, T>(
    source: &'source str,
    block: &MarkdownBlock<T>,
) -> Vec<(String, &'source str)>
where
    T: Extension,
{
    let mut rows = Rows::default();
    rows.markdown_block(0, block);
    rows.rows
        .into_iter()
        .map(|row| (row.label, &source[row.span]))
        .collect()
}

fn block_outline<T>(block: &MarkdownBlock<T>) -> String
where
    T: Extension,
{
    let mut rows = Rows::default();
    rows.markdown_block(0, block);
    outline(&rows.rows)
}

fn document_outline<T>(document: &Document<T>) -> String
where
    T: Extension,
{
    let mut rows = Rows::default();
    rows.document(document);
    outline(&rows.rows)
}

/// Each node of the header of the first comment in the markdown block that
/// `pick` takes from the body of `source`, with the source text it covers.
fn header_spans(
    source: &str,
    pick: fn(&MarkdownBlock<With<Comments>>) -> &MarkdownBlock<With<Comments>>,
) -> Vec<(String, String)> {
    let document = parse_with::<Comments>(source, &ParseOptions::strict()).unwrap();
    let comment = code_blocks(pick(&document.body))[0]
        .kind
        .0
        .as_deref()
        .and_then(second)
        .unwrap();
    let mut rows = Rows::default();
    rows.mapping(0, &comment.header);
    rows.rows
        .into_iter()
        .map(|row| (row.label, source[row.span].to_owned()))
        .collect()
}

/// What `header_spans` gives for the header `id: abc`, `author: ada`, its
/// second line indented by `indent`.
fn header_rows(indent: &str) -> [(String, String); 5] {
    [
        ("Mapping", format!("id: abc\n{indent}author: ada")),
        ("Pair key=id", "id: abc".to_owned()),
        (r#"Scalar String "abc""#, "abc".to_owned()),
        ("Pair key=author", "author: ada".to_owned()),
        (r#"Scalar String "ada""#, "ada".to_owned()),
    ]
    .map(|(label, text)| (label.to_owned(), text))
}

/// `document` written to JSON, read back, and its extension nodes built
/// again.
fn rebuilt(document: &Document<With<Comments>>) -> Document<With<Comments>> {
    let back: Document<With<Comments>> = from_json(&to_json(document).unwrap()).unwrap();
    rebuild_with::<Comments>(&back, &ParseOptions::strict()).unwrap()
}

/// The outline of a comment's body, then of its header.
fn comment_outline(comment: &Comment<With<Comments>>) -> String {
    let mut rows = Rows::default();
    rows.markdown_block(0, &comment.body);
    rows.mapping(0, &comment.header);
    outline(&rows.rows)
}

/// `levels` comments around `a`, each in the body of the one before: the
/// innermost fence is three backticks, and each one around it one longer.
fn nested_comments(levels: usize) -> String {
    let fences: Vec<String> = (3..levels + 3).map(|length| "`".repeat(length)).collect();
    let mut source = String::new();
    for fence in fences.iter().rev() {
        source.push_str(fence);
        source.push_str("remargin\n---\nid: c\n---\n");
    }
    source.push_str("a\n");
    for fence in &fences {
        source.push_str(fence);
        source.push('\n');
    }
    source
}

/// How many claimed comments nest from `body` down, each the only code block
/// of the body before, and the body of the innermost.
fn comment_depth(body: &MarkdownBlock<With<Comments>>) -> (usize, &MarkdownBlock<With<Comments>>) {
    let mut inner = body;
    let mut depth = 0_usize;
    while let &[block] = code_blocks(inner).as_slice()
        && let Some(comment) = block.kind.0.as_deref().and_then(second)
    {
        depth += 1;
        inner = &comment.body;
    }
    (depth, inner)
}

/// The value of the code block in the first item of the markdown block
/// that the first code block of `read` holds, and the outline of the
/// markdown block it holds.
fn inner_claimed(read: &Document<With<Plain>>) -> (String, Option<String>) {
    let outer = code_blocks(&read.body)[0]
        .kind
        .0
        .as_deref()
        .and_then(first)
        .unwrap();
    let block = code_blocks(first_item(outer))[0];
    (
        block.value.clone(),
        block.kind.0.as_deref().and_then(first).map(block_outline),
    )
}

#[test]
fn an_extension_turns_the_blocks_it_claims_into_its_own_typed_node() {
    let document = parse_with::<Comments>(SOURCE, &ParseOptions::default()).unwrap();
    let blocks = code_blocks(&document.body);
    let comment = blocks[0].kind.0.as_deref().and_then(second).unwrap();
    assert_eq!(
        block_outline(&comment.body),
        r#"MarkdownBlock
  Section
    Paragraph
      Line
        Text "Looks good."
"#
    );
    let mut rows = Rows::default();
    rows.mapping(0, &comment.header);
    assert_eq!(
        outline(&rows.rows),
        r#"Mapping
  Pair key=id
    Scalar String "abc"
  Pair key=author
    Scalar String "ada"
  Pair key=type
    Scalar String "human"
  Pair key=to
    Sequence
      Scalar String "grace"
  Pair key=reactions
    Mapping
      Pair key=thumbs
        Sequence
          Mapping
            Pair key=author
              Scalar String "grace"
            Pair key=ts
              Scalar DateTime 2026-10-01T10:00:00Z
  Pair key=ack
    Sequence
      Scalar String "grace@2026-10-01T10:00:00Z"
"#
    );
    assert_eq!(blocks[1].language.as_deref(), Some("rust"));
    assert!(blocks[1].kind.0.is_none());
}

#[test]
fn a_block_the_extension_claims_but_cannot_read_stays_a_plain_code_block() {
    let source = "# Notes\n\n```remargin\njust text, no header\n```\n";
    let document = parse_with::<Comments>(source, &ParseOptions::default()).unwrap();
    assert_eq!(
        document_outline(&document),
        r#"Document
  MarkdownBlock
    Section
      Heading
        Text "Notes"
      CodeBlock language="remargin" value="just text, no header"
"#
    );
}

#[test]
fn a_strict_parse_fails_with_the_error_of_a_claim_that_cannot_be_read() {
    let source = "```remargin\nno header\n```\n";
    let error = parse_with::<Comments>(source, &ParseOptions::strict()).unwrap_err();
    assert_eq!(error.message, NO_HEADER);
    assert_eq!(&source[error.span], "no header");
}

#[test]
fn a_claim_that_cannot_be_read_inside_a_markdown_block_falls_back_or_fails_a_strict_parse() {
    let source = "````markdown\n```remargin\nno header\n```\n````\n";
    let document = parse_with::<Comments>(source, &ParseOptions::default()).unwrap();
    let markdown = code_blocks(&document.body)[0]
        .kind
        .0
        .as_deref()
        .and_then(first)
        .unwrap();
    assert_eq!(
        block_outline(markdown),
        r#"MarkdownBlock
  Section
    CodeBlock language="remargin" value="no header"
"#
    );
    let error = parse_with::<Comments>(source, &ParseOptions::strict()).unwrap_err();
    assert_eq!(error.message, NO_HEADER);
    assert_eq!(&source[error.span], "no header");
}

#[test]
fn a_fenced_block_whose_info_starts_with_markdown_or_md_in_any_case_holds_a_markdown_block() {
    for source in [
        "```markdown\n# A\n```\n",
        "```md\n# A\n```\n",
        "```MD\n# A\n```\n",
        "```Markdown title\n# A\n```\n",
        "~~~markdown\n# A\n~~~\n",
    ] {
        let document = parse(source).unwrap();
        let block = code_blocks(&document.body)[0];
        assert_eq!(block.value, "# A");
        assert_eq!(
            block_outline(block.kind.0.as_deref().unwrap()),
            "MarkdownBlock\n  Section\n    Heading\n      Text \"A\"\n",
            "{source}"
        );
    }
    for source in ["```mdx\n# A\n```\n", "```text markdown\n# A\n```\n"] {
        let document = parse(source).unwrap();
        assert!(code_blocks(&document.body)[0].kind.0.is_none(), "{source}");
    }
}

#[test]
fn a_markdown_block_inside_a_markdown_block_is_read_the_same_way() {
    let source = "````markdown\n```md\n# B\n```\n````\n";
    let document = parse(source).unwrap();
    let outer = code_blocks(&document.body)[0].kind.0.as_deref().unwrap();
    assert_eq!(
        block_outline(outer),
        r##"MarkdownBlock
  Section
    CodeBlock language="md" value="# B" claimed
"##
    );
    let inner = code_blocks(outer)[0].kind.0.as_deref().unwrap();
    assert_eq!(
        block_outline(inner),
        "MarkdownBlock\n  Section\n    Heading\n      Text \"B\"\n"
    );
}

#[test]
fn a_comment_inside_a_markdown_block_is_still_a_comment() {
    let source = "````markdown\n```remargin\n---\nid: abc\n---\nInside.\n```\n````\n";
    let document = parse_with::<Comments>(source, &ParseOptions::default()).unwrap();
    let markdown = code_blocks(&document.body)[0]
        .kind
        .0
        .as_deref()
        .and_then(first)
        .unwrap();
    let comment = code_blocks(markdown)[0]
        .kind
        .0
        .as_deref()
        .and_then(second)
        .unwrap();
    assert_eq!(
        block_outline(&comment.body),
        r#"MarkdownBlock
  Section
    Paragraph
      Line
        Text "Inside."
"#
    );
}

#[test]
fn a_tree_holding_a_comment_inside_a_markdown_block_clones_compares_and_prints() {
    let source = "````markdown\n```remargin\n---\nid: abc\n---\nInside.\n```\n````\n";
    let document = parse_with::<Comments>(source, &ParseOptions::default()).unwrap();
    let cloned = document.clone();
    assert_eq!(cloned, document);
    let markdown = code_blocks(&cloned.body)[0]
        .kind
        .0
        .as_deref()
        .and_then(first)
        .unwrap();
    assert!(
        code_blocks(markdown)[0]
            .kind
            .0
            .as_deref()
            .and_then(second)
            .is_some()
    );
    let printed = format!("{cloned:?}");
    assert_eq!(printed, format!("{document:?}"));
    assert!(printed.contains("Comment {"));
    assert!(printed.contains("\"Inside.\""));
}

#[test]
fn two_extensions_claim_a_whole_document_and_the_comments_inside_it() {
    let source = "\
+++
title = \"Parser\"
+++
# Ticket

```remargin
---
id: abc
---
Top level.
```

````md
```remargin
---
id: def
---
Inside.
```
````
";
    let document = parse_with::<(Comments, Tickets)>(source, &ParseOptions::default()).unwrap();
    let ticket = document
        .kind
        .0
        .as_deref()
        .and_then(second)
        .and_then(second)
        .unwrap();
    assert_eq!(
        ticket,
        &Ticket {
            claimed: 2,
            title: "Parser".to_owned(),
        }
    );
    let blocks = code_blocks(&document.body);
    let top = blocks[0]
        .kind
        .0
        .as_deref()
        .and_then(second)
        .and_then(first)
        .unwrap();
    assert!(block_outline(&top.body).ends_with("Text \"Top level.\"\n"));
    let markdown = blocks[1].kind.0.as_deref().and_then(first).unwrap();
    let inside = code_blocks(markdown)[0]
        .kind
        .0
        .as_deref()
        .and_then(second)
        .and_then(first)
        .unwrap();
    assert!(block_outline(&inside.body).ends_with("Text \"Inside.\"\n"));
}

#[test]
fn the_first_extension_listed_that_claims_a_construct_gets_it() {
    let source =
        "```remargin\n---\nid: abc\n---\n```\n\n```rust\nfn main() {}\n```\n\n```md\n# A\n```\n";
    let options = ParseOptions::default();

    let fences_first = parse_with::<(Fences, Comments)>(source, &options).unwrap();
    let fenced = code_blocks(&fences_first.body);
    let fence = |block: &CodeBlock<With<(Fences, Comments)>>| {
        block
            .kind
            .0
            .as_deref()
            .and_then(second)
            .and_then(first)
            .cloned()
    };
    assert_eq!(fence(fenced[0]).as_deref(), Some("remargin"));
    assert_eq!(fence(fenced[1]).as_deref(), Some("rust"));
    assert!(fenced[2].kind.0.as_deref().and_then(first).is_some());

    let comments_first = parse_with::<(Comments, Fences)>(source, &options).unwrap();
    let commented = code_blocks(&comments_first.body);
    let node = |block: &CodeBlock<With<(Comments, Fences)>>| {
        block.kind.0.as_deref().and_then(second).cloned()
    };
    assert!(node(commented[0]).as_ref().and_then(first).is_some());
    assert_eq!(
        node(commented[1])
            .as_ref()
            .and_then(second)
            .map(String::as_str),
        Some("rust")
    );
    assert!(commented[2].kind.0.as_deref().and_then(first).is_some());
}

#[test]
fn a_markdown_block_inside_a_nested_item_holds_the_section_of_its_heading() {
    let document = parse("- - ```markdown\n    # A\n    ```\n").unwrap();
    let block = code_blocks(first_item(first_item(&document.body)))[0];
    assert_eq!(
        block_outline(block.kind.0.as_deref().unwrap()),
        "MarkdownBlock\n  Section\n    Heading\n      Text \"A\"\n"
    );
}

#[test]
fn a_markdown_block_inside_a_nested_item_reads_as_it_does_at_the_root() {
    for body in [
        "# A\ntext\n\n## B\n",
        "- a\n  - b\n\n    more b\n",
        "    indented code\n\n    more code\n",
        "```rust\nfn main() {}\n```\n",
        "| a | b |\n|---|---|\n| 1 | 2 |\n",
        "+---+---+\n| a | b |\n+---+---+\n",
        "Name  Age\n----  ---\nAda   36\n",
        "  # two columns in\n",
    ] {
        let root = format!("````markdown\n{body}````\n");
        let nested = format!("- - ````markdown\n{}    ````\n", indented(body, "    "));
        let root_document = parse(&root).unwrap();
        let nested_document = parse(&nested).unwrap();
        let at_root = code_blocks(&root_document.body)[0]
            .kind
            .0
            .as_deref()
            .unwrap();
        let in_item = code_blocks(first_item(first_item(&nested_document.body)))[0]
            .kind
            .0
            .as_deref()
            .unwrap();
        assert_eq!(block_outline(in_item), block_outline(at_root), "{body}");
        for (label, text) in block_spans(&nested, in_item) {
            if label.starts_with("Text ") {
                assert_eq!(label, format!("Text {text:?}"), "{body}");
            }
        }
    }
}

#[test]
fn a_line_indented_less_than_the_fence_loses_only_the_spaces_it_has() {
    let source = "   ```markdown\n  # A\n text\n   ```\n";
    let document = parse(source).unwrap();
    let block = code_blocks(&document.body)[0];
    assert_eq!(block.value, "# A\ntext");
    let spans = block_spans(source, block.kind.0.as_deref().unwrap());
    let expected = [
        ("MarkdownBlock", "  # A\n text"),
        ("Section", "# A\n text"),
        ("Heading", "# A"),
        (r#"Text "A""#, "A"),
        ("Paragraph", "text"),
        ("Line", "text"),
        (r#"Text "text""#, "text"),
    ];
    assert_eq!(
        spans
            .iter()
            .map(|(label, text)| (label.as_str(), *text))
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn tab_stops_in_a_markdown_block_count_from_its_fence_as_in_its_value() {
    let source = "- x\n\n    ```md\n   -\ty\n        z\n    ```\n";
    let options = ParseOptions::default();
    let document = parse_with::<Plain>(source, &options).unwrap();
    let claimed = |read: &Document<With<Plain>>| {
        let block = code_blocks(first_item(&read.body))[0];
        (
            block.value.clone(),
            block.kind.0.as_deref().and_then(first).map(block_outline),
        )
    };
    let (value, markdown) = claimed(&document);
    assert_eq!(value, "-\ty\n    z");
    assert_eq!(markdown, Some(block_outline(&parse(&value).unwrap().body)));
    assert_eq!(
        markdown.as_deref(),
        Some(
            r#"MarkdownBlock
  Section
    List
      Item
        MarkdownBlock
          Section
            Paragraph
              Line
                Text "y"
              Line
                Text "z"
"#
        )
    );
    let back: Document<With<Plain>> = from_json(&to_json(&document).unwrap()).unwrap();
    let again = rebuild_with::<Plain>(&back, &options).unwrap();
    assert_eq!(claimed(&again), claimed(&document));
    let written = "- x\n\n  ```md\n  - y\n    z\n  ```\n";
    assert_eq!(write(&document, &WriteOptions::default()).unwrap(), written);
    assert_eq!(write(&again, &WriteOptions::default()).unwrap(), written);
}

#[test]
fn a_simple_table_in_a_markdown_block_counts_its_tab_stops_from_the_fence() {
    let source = "- x\n\n  ```md\n  Name\tAge\n  ----\t-\t---\n  Ada\t\t36\n  ```\n";
    let document = parse(source).unwrap();
    let block = code_blocks(first_item(&document.body))[0];
    let claimed = block_outline(block.kind.0.as_deref().unwrap());
    assert_eq!(claimed, block_outline(&parse(&block.value).unwrap().body));
    assert_eq!(
        claimed,
        r#"MarkdownBlock
  Section
    Table format=Simple alignments=Default,Left,Default
      TableRow head
        TableCell
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "Name"
        TableCell
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "Age"
        TableCell
          MarkdownBlock
      TableRow
        TableCell
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "Ada"
        TableCell
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "36"
        TableCell
          MarkdownBlock
"#
    );
}

#[test]
fn a_markdown_block_at_column_0_counts_its_tab_stops_from_the_start_of_the_line() {
    let source = "```md\n-\ty\n    z\n\n\t\tcode\n\nName\tAge\n----\t---\nAda\t\t36\n```\n";
    let document = parse(source).unwrap();
    assert_eq!(
        block_outline(code_blocks(&document.body)[0].kind.0.as_deref().unwrap()),
        r#"MarkdownBlock
  Section
    List loose
      Item
        MarkdownBlock
          Section
            Paragraph
              Line
                Text "y"
              Line
                Text "z"
            CodeBlock value="code"
    Table format=Simple alignments=Default,Default
      TableRow head
        TableCell
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "Name"
        TableCell
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "Age"
      TableRow
        TableCell
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "Ada"
        TableCell
          MarkdownBlock
            Section
              Paragraph
                Line
                  Text "36"
"#
    );
}

#[test]
fn a_markdown_block_in_a_markdown_block_reads_as_its_value_when_a_tab_comes_before_its_fence() {
    let source = "  ````md\n  -\t```md\n          x\n      ```\n  ````\n";
    let options = ParseOptions::default();
    let document = parse_with::<Plain>(source, &options).unwrap();
    let (value, markdown) = inner_claimed(&document);
    assert_eq!(value, "    x");
    assert_eq!(markdown, Some(block_outline(&parse(&value).unwrap().body)));
    assert_eq!(
        markdown.as_deref(),
        Some("MarkdownBlock\n  Section\n    CodeBlock value=\"x\"\n")
    );
    let back: Document<With<Plain>> = from_json(&to_json(&document).unwrap()).unwrap();
    let again = rebuild_with::<Plain>(&back, &options).unwrap();
    assert_eq!(inner_claimed(&again), inner_claimed(&document));
    let written = "`````md\n- ````md\n  ```\n  x\n  ```\n  ````\n`````\n";
    assert_eq!(write(&document, &WriteOptions::default()).unwrap(), written);
    assert_eq!(write(&again, &WriteOptions::default()).unwrap(), written);
}

#[test]
fn a_markdown_block_in_a_markdown_block_reads_as_its_value_with_any_white_space_before_its_fence() {
    let options = ParseOptions::default();
    for column in 0..4 {
        let pad = " ".repeat(column);
        for gap in ["\t", " ", "  ", "   "] {
            let source =
                format!("{pad}````md\n{pad}-{gap}```md\n{pad}        x\n{pad}    ```\n{pad}````\n");
            let document = parse_with::<Plain>(&source, &options).unwrap();
            let (value, markdown) = inner_claimed(&document);
            assert_eq!(
                markdown,
                Some(block_outline(&parse(&value).unwrap().body)),
                "{source:?}"
            );
            let back: Document<With<Plain>> = from_json(&to_json(&document).unwrap()).unwrap();
            let again = rebuild_with::<Plain>(&back, &options).unwrap();
            assert_eq!(
                inner_claimed(&again),
                inner_claimed(&document),
                "{source:?}"
            );
            assert_eq!(
                write(&again, &WriteOptions::default()).unwrap(),
                write(&document, &WriteOptions::default()).unwrap(),
                "{source:?}"
            );
        }
    }
}

#[test]
fn every_span_inside_a_nested_markdown_block_covers_its_text_in_the_source() {
    let source = "- - ```markdown\n    # A\n    text\n    ```\n";
    let document = parse(source).unwrap();
    let block = code_blocks(first_item(first_item(&document.body)))[0];
    let spans = block_spans(source, block.kind.0.as_deref().unwrap());
    let expected = [
        ("MarkdownBlock", "    # A\n    text"),
        ("Section", "# A\n    text"),
        ("Heading", "# A"),
        (r#"Text "A""#, "A"),
        ("Paragraph", "text"),
        ("Line", "text"),
        (r#"Text "text""#, "text"),
    ];
    assert_eq!(
        spans
            .iter()
            .map(|(label, text)| (label.as_str(), *text))
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn a_comment_inside_a_nested_item_has_the_same_body_as_at_the_root() {
    let body = "---\nid: abc\n---\n# Title\n\n- point\n\n      code\n";
    let root = format!("```remargin\n{body}```\n");
    let nested = format!("- - ```remargin\n{}    ```\n", indented(body, "    "));
    let options = ParseOptions::strict();
    let root_document = parse_with::<Comments>(&root, &options).unwrap();
    let nested_document = parse_with::<Comments>(&nested, &options).unwrap();
    let at_root = code_blocks(&root_document.body)[0]
        .kind
        .0
        .as_deref()
        .and_then(second)
        .unwrap();
    let in_item = code_blocks(first_item(first_item(&nested_document.body)))[0]
        .kind
        .0
        .as_deref()
        .and_then(second)
        .unwrap();
    assert_eq!(
        block_outline(&at_root.body),
        r#"MarkdownBlock
  Section
    Heading
      Text "Title"
    List loose
      Item
        MarkdownBlock
          Section
            Paragraph
              Line
                Text "point"
            CodeBlock value="code"
"#
    );
    assert_eq!(block_outline(&in_item.body), block_outline(&at_root.body));
}

#[test]
fn an_extension_may_claim_a_block_quote_an_html_block_or_a_thematic_break() {
    let document = parse_with::<Raw>("> q\n\n<div>\n\n***\n", &ParseOptions::default()).unwrap();
    let labels: Vec<Option<&str>> = document.body.sections[0]
        .blocks
        .iter()
        .map(|block| {
            let kind = match block {
                Block::BlockQuote(quote) => &quote.kind,
                Block::Html(html) => &html.kind,
                Block::ThematicBreak(thematic_break) => &thematic_break.kind,
                Block::CodeBlock(_) | Block::List(_) | Block::Paragraph(_) | Block::Table(_) => {
                    return None;
                }
            };
            kind.0.as_deref().and_then(second).map(String::as_str)
        })
        .collect();
    assert_eq!(labels, [Some("quote q"), Some("html <div>"), Some("break")]);
}

#[test]
fn an_extension_may_claim_emphasis_strong_inline_html_or_a_line_break() {
    let document = parse_with::<Marks>("*a* **b** <br>\\\nc\n", &ParseOptions::default()).unwrap();
    let first = document.body.sections[0]
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(paragraph) => paragraph.lines.first(),
            Block::BlockQuote(_)
            | Block::CodeBlock(_)
            | Block::Html(_)
            | Block::List(_)
            | Block::Table(_)
            | Block::ThematicBreak(_) => None,
        })
        .unwrap();
    let labels: Vec<Option<&str>> = first
        .children
        .iter()
        .map(|inline| {
            let kind = match inline {
                Inline::Code(code) => &code.kind,
                Inline::Emphasis(emphasis) => &emphasis.kind,
                Inline::Html(html) => &html.kind,
                Inline::Image(image) => &image.kind,
                Inline::LineBreak(line_break) => &line_break.kind,
                Inline::Link(link) => &link.kind,
                Inline::Strong(strong) => &strong.kind,
                Inline::Text(text) => &text.kind,
            };
            kind.0.as_deref().and_then(second).map(String::as_str)
        })
        .collect();
    assert_eq!(
        labels,
        [
            Some("emphasis"),
            None,
            Some("strong"),
            None,
            Some("html <br>"),
            Some("break")
        ]
    );
}

#[test]
fn a_markdown_block_in_a_grid_cell_reads_only_that_cells_text() {
    let source = "\
+-----------+---+
| ```md     | x |
| # A       | y |
| text      | z |
| ```       |   |
+-----------+---+
";
    let document = parse(source).unwrap();
    let block = code_blocks(&first_cell(&document.body).body)[0];
    assert_eq!(block.value, "# A\ntext");
    let claimed = block.kind.0.as_deref().unwrap();
    assert_eq!(
        block_outline(claimed),
        r#"MarkdownBlock
  Section
    Heading
      Text "A"
    Paragraph
      Line
        Text "text"
"#
    );
    let spans = block_spans(source, claimed);
    let expected = [
        ("MarkdownBlock", "# A       | y |\n| text"),
        ("Section", "# A       | y |\n| text"),
        ("Heading", "# A"),
        (r#"Text "A""#, "A"),
        ("Paragraph", "text"),
        ("Line", "text"),
        (r#"Text "text""#, "text"),
    ];
    assert_eq!(
        spans
            .iter()
            .map(|(label, text)| (label.as_str(), *text))
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn a_markdown_block_in_a_grid_cell_counts_its_columns_from_the_cell() {
    let grid = in_grid_cell("```md\n    code\n```");
    let in_item = format!("- {}", &indented(&grid, "  ")[2..]);
    for (source, in_list) in [(grid, false), (in_item, true)] {
        let document = parse(&source).unwrap();
        let body = if in_list {
            first_item(&document.body)
        } else {
            &document.body
        };
        let block = code_blocks(&first_cell(body).body)[0];
        assert_eq!(
            block_outline(block.kind.0.as_deref().unwrap()),
            r#"MarkdownBlock
  Section
    CodeBlock value="code"
"#,
            "{source}"
        );
    }
}

#[test]
fn tab_stops_in_a_markdown_block_in_a_grid_cell_count_from_its_fence_as_in_its_value() {
    let source = "+-------------+---+\n| a           | x |\n|   ```md     | x |\n|   -\ty       | x |\n|             | x |\n|           z | x |\n|   ```       | x |\n+-------------+---+\n";
    let options = ParseOptions::default();
    let document = parse_with::<Plain>(source, &options).unwrap();
    let claimed = |read: &Document<With<Plain>>| {
        let block = code_blocks(&first_cell(&read.body).body)[0];
        (
            block.value.clone(),
            block.kind.0.as_deref().and_then(first).map(block_outline),
        )
    };
    let (value, markdown) = claimed(&document);
    assert_eq!(value, "-\ty\n\n        z");
    assert_eq!(markdown, Some(block_outline(&parse(&value).unwrap().body)));
    assert_eq!(
        markdown.as_deref(),
        Some(
            r#"MarkdownBlock
  Section
    List loose
      Item
        MarkdownBlock
          Section
            Paragraph
              Line
                Text "y"
            CodeBlock value="z"
"#
        )
    );
    let back: Document<With<Plain>> = from_json(&to_json(&document).unwrap()).unwrap();
    let again = rebuild_with::<Plain>(&back, &options).unwrap();
    assert_eq!(claimed(&again), claimed(&document));
    let written = "+--------+-----+\n| a      | x   |\n|        | x   |\n| ````md | x   |\n| - y    | x   |\n|        | x   |\n|   ```  | x   |\n|   z    |     |\n|   ```  |     |\n| ````   |     |\n+--------+-----+\n";
    assert_eq!(write(&document, &WriteOptions::default()).unwrap(), written);
    assert_eq!(write(&again, &WriteOptions::default()).unwrap(), written);
}

#[test]
fn a_markdown_block_in_a_grid_cell_reads_as_its_value_with_its_fence_at_any_column() {
    let options = ParseOptions::default();
    for column in 0..8 {
        let pad = " ".repeat(column);
        // The item's text starts at column 4, so it holds the fences at 4 to 7.
        let source = in_grid_cell(&format!(
            "-   a\n\n{pad}```md\n{pad}-\ty\n\n{pad}        z\n{pad}```"
        ));
        let document = parse_with::<Plain>(&source, &options).unwrap();
        let claimed = |read: &Document<With<Plain>>| {
            let body = &first_cell(&read.body).body;
            let block = if column < 4 {
                code_blocks(body)[0]
            } else {
                code_blocks(first_item(body))[0]
            };
            (
                block.value.clone(),
                block.kind.0.as_deref().and_then(first).map(block_outline),
            )
        };
        let (value, markdown) = claimed(&document);
        assert_eq!(value, "-\ty\n\n        z", "{source}");
        assert_eq!(
            markdown,
            Some(block_outline(&parse(&value).unwrap().body)),
            "{source}"
        );
        let back: Document<With<Plain>> = from_json(&to_json(&document).unwrap()).unwrap();
        let again = rebuild_with::<Plain>(&back, &options).unwrap();
        assert_eq!(claimed(&again), claimed(&document), "{source}");
        assert_eq!(
            write(&again, &WriteOptions::default()).unwrap(),
            write(&document, &WriteOptions::default()).unwrap(),
            "{source}"
        );
    }
}

#[test]
fn a_comment_in_a_grid_cell_has_the_same_body_as_at_the_root() {
    let body = "---\nid: abc\n---\n# Title\n\n- point\n\n      code\n";
    let root = format!("```remargin\n{body}```\n");
    let grid = in_grid_cell(&format!("```remargin\n{body}```"));
    let options = ParseOptions::strict();
    let root_document = parse_with::<Comments>(&root, &options).unwrap();
    let grid_document = parse_with::<Comments>(&grid, &options).unwrap();
    let at_root = code_blocks(&root_document.body)[0]
        .kind
        .0
        .as_deref()
        .and_then(second)
        .unwrap();
    let in_cell = code_blocks(&first_cell(&grid_document.body).body)[0]
        .kind
        .0
        .as_deref()
        .and_then(second)
        .unwrap();
    assert!(block_outline(&at_root.body).contains("Text \"Title\""));
    assert_eq!(block_outline(&in_cell.body), block_outline(&at_root.body));
    let header = |comment: &Comment<With<Comments>>| {
        let mut rows = Rows::default();
        rows.mapping(0, &comment.header);
        outline(&rows.rows)
    };
    assert_eq!(header(in_cell), header(at_root));
}

#[test]
fn a_comment_header_over_several_lines_of_a_grid_cell_reads_as_at_the_root() {
    let written = "```remargin\n---\nid: abc\nauthor: ada\n---\nBody.\n```";
    let grid = in_grid_cell(written);
    assert_eq!(
        header_spans(&grid, |body| &first_cell(body).body),
        [
            ("Mapping", "id: abc     | x |\n| author: ada"),
            ("Pair key=id", "id: abc"),
            (r#"Scalar String "abc""#, "abc"),
            ("Pair key=author", "author: ada"),
            (r#"Scalar String "ada""#, "ada"),
        ]
        .map(|(label, text)| (label.to_owned(), text.to_owned())),
        "{grid}"
    );
    assert_eq!(
        header_spans(&format!("{written}\n"), |body| body),
        [
            ("Mapping", "id: abc\nauthor: ada"),
            ("Pair key=id", "id: abc"),
            (r#"Scalar String "abc""#, "abc"),
            ("Pair key=author", "author: ada"),
            (r#"Scalar String "ada""#, "ada"),
        ]
        .map(|(label, text)| (label.to_owned(), text.to_owned()))
    );
    let refused = in_grid_cell("```remargin\n---\nid: abc\nauthor: @ada\n---\n```");
    let error = parse_with::<Comments>(&refused, &ParseOptions::strict()).unwrap_err();
    assert_eq!(error.span.start, refused.find('@').unwrap(), "{error}");
    assert_eq!(&refused[error.span], "@");
}

#[test]
fn a_toml_comment_header_over_several_lines_of_a_grid_cell_reads_as_at_the_root() {
    let written = "```remargin\n+++\nid = \"abc\"\nauthor = \"ada\"\n+++\nBody.\n```";
    let grid = in_grid_cell(written);
    assert_eq!(
        header_spans(&grid, |body| &first_cell(body).body),
        [
            ("Mapping", "id = \"abc\"     | x |\n| author = \"ada\""),
            ("Pair key=id", "id = \"abc\""),
            (r#"Scalar String "abc""#, "\"abc\""),
            ("Pair key=author", "author = \"ada\""),
            (r#"Scalar String "ada""#, "\"ada\""),
        ]
        .map(|(label, text)| (label.to_owned(), text.to_owned())),
        "{grid}"
    );
    assert_eq!(
        header_spans(&format!("{written}\n"), |body| body),
        [
            ("Mapping", "id = \"abc\"\nauthor = \"ada\""),
            ("Pair key=id", "id = \"abc\""),
            (r#"Scalar String "abc""#, "\"abc\""),
            ("Pair key=author", "author = \"ada\""),
            (r#"Scalar String "ada""#, "\"ada\""),
        ]
        .map(|(label, text)| (label.to_owned(), text.to_owned()))
    );
}

#[test]
fn a_comment_header_in_a_list_item_indented_with_tabs_reads_as_at_the_root() {
    let root = "```remargin\n---\nid: abc\nauthor: ada\n---\nBody.\n```\n";
    assert_eq!(header_spans(root, |body| body), header_rows(""));
    for (source, indent) in [
        (
            "- ```remargin\n  ---\n  id: abc\n  author: ada\n  ---\n  Body.\n  ```\n",
            "  ",
        ),
        (
            "-\t```remargin\n\t---\n\tid: abc\n\tauthor: ada\n\t---\n\tBody.\n\t```\n",
            "\t",
        ),
    ] {
        assert_eq!(
            header_spans(source, first_item),
            header_rows(indent),
            "{source:?}"
        );
    }
}

#[test]
fn a_comment_in_a_markdown_block_in_a_list_item_has_the_same_header_as_at_the_root() {
    let written = "````markdown\n```remargin\n---\nid: abc\nauthor: ada\n---\nBody.\n```\n````\n";
    for (marker, indent) in [("- ", "  "), ("-\t", "\t")] {
        let source = format!("{marker}{}", &indented(written, indent)[indent.len()..]);
        let spans = header_spans(&source, |body| {
            code_blocks(first_item(body))[0]
                .kind
                .0
                .as_deref()
                .and_then(first)
                .unwrap()
        });
        assert_eq!(spans, header_rows(indent), "{source:?}");
    }
}

#[test]
fn a_markdown_block_in_a_pipe_cell_is_read_from_the_source() {
    let source = "| ```md | x |\n|---|---|\n";
    let document = parse(source).unwrap();
    let block = code_blocks(&first_cell(&document.body).body)[0];
    assert_eq!(block.content, 7..7);
    let claimed = block.kind.0.as_deref().unwrap();
    assert_eq!(block_outline(claimed), "MarkdownBlock\n");
    assert_eq!(claimed.span, block.content);
}

#[test]
fn a_markdown_block_in_a_grid_cell_inside_a_markdown_block_in_a_grid_cell_reads_its_own_cell() {
    let inner = in_grid_cell("```md\n# B\ntext\n```");
    let source = in_grid_cell(&format!("```md\n{inner}```"));
    let document = parse(&source).unwrap();
    let outer = code_blocks(&first_cell(&document.body).body)[0]
        .kind
        .0
        .as_deref()
        .unwrap();
    let block = code_blocks(&first_cell(outer).body)[0];
    assert_eq!(
        block_outline(block.kind.0.as_deref().unwrap()),
        r#"MarkdownBlock
  Section
    Heading
      Text "B"
    Paragraph
      Line
        Text "text"
"#
    );
}

#[test]
fn an_extension_that_claims_a_grid_cell_reads_only_that_cells_text() {
    let source = in_grid_cell("# A\n\ntext");
    let document = parse_with::<CellText>(&source, &ParseOptions::default()).unwrap();
    let cell = first_cell(&document.body);
    let read = cell.kind.0.as_deref().and_then(second).unwrap();
    assert_eq!(
        block_outline(read),
        r#"MarkdownBlock
  Section
    Heading
      Text "A"
    Paragraph
      Line
        Text "text"
"#
    );
    assert_eq!(block_outline(read), block_outline(&cell.body));
}

#[test]
fn a_comment_is_written_back_as_its_own_text_and_reads_back_to_the_same_node() {
    let source =
        "# Notes\n\n```remargin\n---\nid: abc\nauthor: ada\n---\nLooks *good*.\n\n- one\n```\n";
    let document = parse_with::<Comments>(source, &ParseOptions::strict()).unwrap();
    let text = write(&document, &WriteOptions::default()).unwrap();
    assert_eq!(text, source);
    let again = parse_with::<Comments>(&text, &ParseOptions::strict()).unwrap();
    let comment = |read: &Document<With<Comments>>| {
        code_blocks(&read.body)[0]
            .kind
            .0
            .as_deref()
            .and_then(second)
            .cloned()
    };
    assert!(comment(&document).is_some());
    assert_eq!(comment(&again), comment(&document));
}

#[test]
fn an_extension_that_only_reads_writes_the_plain_node_in_archies_form() {
    let fenced = parse_with::<Fences>("Code:\n\n    indented\n", &ParseOptions::default()).unwrap();
    assert!(code_blocks(&fenced.body)[0].kind.0.is_some());
    assert_eq!(
        write(&fenced, &WriteOptions::default()).unwrap(),
        "Code:\n\n```\nindented\n```\n"
    );
    let marked = parse_with::<Marks>("_a_ __b__ <br>  \nc\n", &ParseOptions::default()).unwrap();
    assert_eq!(
        write(&marked, &WriteOptions::default()).unwrap(),
        "*a* **b** <br>\\\nc\n"
    );
}

#[test]
fn a_comment_in_a_grid_cell_is_built_again_from_json() {
    let grid = in_grid_cell("```remargin\n---\nid: abc\n---\n# Title\n\n- point\n```");
    let document = parse_with::<Comments>(&grid, &ParseOptions::strict()).unwrap();
    let again = rebuilt(&document);
    assert!(document_outline(&again).contains("Table format=Grid"));
    let comment = |read: &Document<With<Comments>>| {
        code_blocks(&first_cell(&read.body).body)[0]
            .kind
            .0
            .as_deref()
            .and_then(second)
            .map(comment_outline)
    };
    assert!(comment(&document).unwrap().contains("Text \"Title\""));
    assert_eq!(comment(&again), comment(&document));
}

#[test]
fn a_markdown_block_in_a_list_item_is_built_again_from_json() {
    let source = "- ```markdown\n  # A\n\n  text\n  ```\n";
    let document = parse_with::<Comments>(source, &ParseOptions::strict()).unwrap();
    let markdown = |read: &Document<With<Comments>>| {
        code_blocks(first_item(&read.body))[0]
            .kind
            .0
            .as_deref()
            .and_then(first)
            .map(block_outline)
    };
    assert!(markdown(&document).unwrap().contains("Text \"text\""));
    assert_eq!(markdown(&rebuilt(&document)), markdown(&document));
}

#[test]
fn comments_nest_the_block_cap_deep_and_the_comment_past_it_stays_a_code_block() {
    on_two_megabytes(|| {
        let source = nested_comments(1000);
        let document = parse_with::<Comments>(&source, &ParseOptions::default()).unwrap();
        let (depth, inner) = comment_depth(&document.body);
        assert_eq!(depth, BLOCK_CAP);
        assert!(matches!(code_blocks(inner).as_slice(),
            [code] if code.kind.0.is_none() && code.language.as_deref() == Some("remargin")));
        write(&document, &WriteOptions::default()).unwrap();
        let copy = document.clone();
        assert_eq!(copy, document);
        let printed = format!("{document:?}");
        assert_eq!(printed.matches("Comment {").count(), BLOCK_CAP);
        drop(copy);
        drop(document);
    });
}

#[test]
fn a_comment_inside_the_block_cap_of_items_stays_a_code_block_and_inside_one_less_is_claimed() {
    on_two_megabytes(|| {
        for (items, claimed) in [(BLOCK_CAP, false), (BLOCK_CAP - 1, true)] {
            let source = format!(
                "{}```remargin\n{}",
                "- ".repeat(items),
                indented("---\nid: c\n---\nx\n```\n", &"  ".repeat(items))
            );
            let document = parse_with::<Comments>(&source, &ParseOptions::default()).unwrap();
            let mut body = &document.body;
            for _ in 0..items {
                body = first_item(body);
            }
            let block = code_blocks(body)[0];
            assert_eq!(block.language.as_deref(), Some("remargin"), "{items}");
            assert_eq!(
                block.kind.0.as_deref().and_then(second).is_some(),
                claimed,
                "{items}"
            );
        }
    });
}
