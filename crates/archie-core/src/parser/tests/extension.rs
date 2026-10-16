//! Extensions declared outside archie's tree: a comment block shaped like
//! remargin's, read into a typed node, and a whole file read as a ticket.

use core::ops::Range;

use super::{Rows, outline};
use crate::parser::ast::{Block, CodeBlock, Document, Mapping, MarkdownBlock, ScalarValue, Value};
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
            Block::List(_) | Block::Paragraph(_) | Block::Table(_) => None,
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
