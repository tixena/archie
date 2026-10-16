//! An extension declared outside archie's tree: a comment block shaped
//! like remargin's, read into a typed node.

use core::ops::Range;

use super::{Rows, outline};
use crate::parser::ast::{Block, CodeBlock, Document, Extension, FencedCode, Mapping};
use crate::parser::error::ParseError;
use crate::parser::{parse_with, read_yaml};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Comments;

#[derive(Debug, Clone, PartialEq)]
struct Comment {
    body: String,
    header: Mapping,
}

impl Extension for Comments {
    type CodeBlock = Comment;

    fn code_block(block: &FencedCode<'_>) -> Option<Result<Comment, ParseError>> {
        (block.info == "remargin").then(|| comment(block))
    }
}

/// A `---` line, the YAML header, a `---` line, then the body.
fn comment(block: &FencedCode<'_>) -> Result<Comment, ParseError> {
    let refused = || {
        ParseError::new(
            "a comment opens with a header between `---` lines",
            block.content.clone(),
        )
    };
    let mut lines = Vec::new();
    let mut at = block.content.start;
    for line in block.source[block.content.clone()].split_inclusive('\n') {
        lines.push(at..at + line.len());
        at += line.len();
    }
    let fence = |line: &Range<usize>| block.source[line.clone()].trim() == "---";
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
        body: block
            .value
            .splitn(3, "---\n")
            .nth(2)
            .unwrap_or("")
            .to_owned(),
        header: read_yaml(block.source, open.end..close.start)?,
    })
}

fn code_blocks(document: &Document<Comments>) -> Vec<&CodeBlock<Comments>> {
    document
        .body
        .children
        .iter()
        .filter_map(|block| match block {
            Block::CodeBlock(code_block) => Some(code_block),
            Block::List(_) | Block::Paragraph(_) | Block::Section(_) | Block::Table(_) => None,
        })
        .collect()
}

#[test]
fn an_extension_turns_the_blocks_it_claims_into_its_own_typed_node() {
    let document = parse_with::<Comments>(SOURCE).unwrap();
    let blocks = code_blocks(&document);
    let comment = blocks[0].kind.as_ref().unwrap();
    assert_eq!(comment.body, "Looks good.");
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
    assert_eq!(blocks[1].kind, None);
}

#[test]
fn a_block_the_extension_claims_but_cannot_read_fails_the_parse() {
    let source = "```remargin\nno header\n```\n";
    let error = parse_with::<Comments>(source).unwrap_err();
    assert_eq!(
        error.message,
        "a comment opens with a header between `---` lines"
    );
    assert_eq!(&source[error.span], "no header");
}
