//! Extensions declared outside archie's tree: a comment block shaped like
//! remargin's, read into a typed node, and a whole file read as a ticket.

use core::ops::Range;

use super::{Rows, outline};
use crate::parser::ast::{
    Block, CodeBlock, Document, Inline, Mapping, MarkdownBlock, ScalarValue, Value,
};
use crate::parser::error::ParseError;
use crate::parser::extension::{Construct, Context, Either, Extension, With};
use crate::parser::{ParseOptions, parse, parse_with, read_yaml};

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

const NO_HEADER: &str = "a comment opens with a header between `---` lines";

#[derive(Debug, Clone, PartialEq)]
struct Comments;

/// A comment: its YAML header, and its body read as markdown with the
/// extensions the file is parsed with.
#[derive(Debug, Clone, PartialEq)]
struct Comment<T>
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
        (block.info == "remargin").then(|| comment(block, context))
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
        let pair = document.frontmatter.as_ref()?.mapping.pairs.get("title")?;
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
}

/// Claims every code block, as its info string.
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
        Some(Ok(block.info.clone()))
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
}

/// A `---` line, the YAML header, a `---` line, then the body.
fn comment<T>(block: &CodeBlock<T>, context: &Context<'_>) -> Result<Comment<T>, ParseError>
where
    T: Extension,
{
    let refused = || ParseError::new(NO_HEADER, block.content.clone());
    let mut lines = Vec::new();
    let mut at = block.content.start;
    for line in context.source[block.content.clone()].split_inclusive('\n') {
        lines.push(at..at + line.len());
        at += line.len();
    }
    let fence = |line: &Range<usize>| context.source[line.clone()].trim() == "---";
    let open = lines
        .first()
        .filter(|line| fence(line))
        .ok_or_else(refused)?;
    let close = lines
        .iter()
        .skip(1)
        .find(|line| fence(line))
        .ok_or_else(refused)?;
    Ok(Comment {
        body: context.markdown::<T>(close.end..block.content.end)?,
        header: read_yaml(context.source, open.end..close.start)?,
    })
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

#[test]
fn an_extension_turns_the_blocks_it_claims_into_its_own_typed_node() {
    let document = parse_with::<Comments>(SOURCE, &ParseOptions::default()).unwrap();
    let blocks = code_blocks(&document.body);
    let comment = blocks[0].kind.0.as_deref().and_then(second).unwrap();
    assert_eq!(
        block_outline(&comment.body),
        r#"MarkdownBlock
  Section
    Heading
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
    assert_eq!(blocks[1].info, "rust");
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
      CodeBlock info="remargin" value="just text, no header"
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
    Heading
    CodeBlock info="remargin" value="no header"
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
    Heading
    CodeBlock info="md" value="# B" claimed
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
    Heading
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
            Heading
            Paragraph
              Line
                Text "point"
            CodeBlock info="" value="code"
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
