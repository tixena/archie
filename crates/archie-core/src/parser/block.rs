//! Body tokens into blocks: sections, paragraphs, lists, tables and code
//! blocks.

use core::marker::PhantomData;
use core::mem;
use core::ops::Range;

use crate::parser::ast::{
    Block, CodeBlock, Extension, FencedCode, Heading, Item, List, MarkdownBlock, Paragraph,
    Section, Table, TableCell, TableRow,
};
use crate::parser::error::ParseError;
use crate::parser::inline::line;
use crate::parser::lexer::{Token, TokenKind, lex};
use crate::parser::table::{RowToken, TableToken};

struct Parser<'src, E> {
    extension: PhantomData<E>,
    pos: usize,
    source: &'src str,
    tokens: Vec<Token>,
}

/// The sections of a `MarkdownBlock` being read: `closed` holds the finished
/// top-level sections, `open` the open ones, outermost first, so a section's
/// depth is its place in `open`.
struct Sections<E>
where
    E: Extension,
{
    closed: Vec<Section<E>>,
    open: Vec<Section<E>>,
}

impl<E> Parser<'_, E>
where
    E: Extension,
{
    /// A cell holds one section, with an empty heading, holding a paragraph
    /// of one line, or nothing when empty.
    fn cell(&self, range: Range<usize>) -> TableCell<E> {
        let mut sections = Sections::new();
        if !range.is_empty() {
            sections.block(Block::Paragraph(Paragraph {
                lines: vec![line(self.source, range.clone())],
                span: range.clone(),
            }));
        }
        TableCell {
            body: MarkdownBlock {
                sections: sections.finish(),
                span: range.clone(),
            },
            span: range,
        }
    }

    /// The heading of the heading line at `written`, whose text is at `text`.
    /// A line with no text gives an empty heading at its start.
    fn heading(&self, text: Range<usize>, written: &Range<usize>) -> Heading {
        if text.is_empty() {
            empty_heading(written.start)
        } else {
            Heading {
                children: line(self.source, text).children,
                span: written.clone(),
            }
        }
    }

    fn item(&mut self, marker: Range<usize>) -> Result<Item<E>, ParseError> {
        let body = self.markdown_block(marker.end)?;
        if self
            .tokens
            .get(self.pos)
            .is_some_and(|token| matches!(token.kind, TokenKind::ItemEnd))
        {
            self.pos += 1;
        }
        let end = body
            .sections
            .last()
            .map_or(marker.end, |section| section.span.end);
        Ok(Item {
            body,
            span: marker.start..end,
        })
    }

    /// Consecutive items make one list. Blank lines between them do not
    /// end it.
    fn list(&mut self, marker: Range<usize>) -> Result<List<E>, ParseError> {
        let start = marker.start;
        let mut items = vec![self.item(marker)?];
        while let Some(next) = self.next_item() {
            items.push(self.item(next)?);
        }
        let end = items.last().map_or(start, |item| item.span.end);
        Ok(List {
            items,
            span: start..end,
        })
    }

    /// The sections up to the end of the enclosing list item, or of the
    /// tokens. It spans its sections, or is empty at `at` when it has none.
    fn markdown_block(&mut self, at: usize) -> Result<MarkdownBlock<E>, ParseError> {
        let mut sections = Sections::new();
        while !self.stops() {
            let Some(token) = self.take() else {
                break;
            };
            let found = match token.kind {
                TokenKind::Blank | TokenKind::ItemEnd => None,
                TokenKind::CodeBlock {
                    content,
                    info,
                    value,
                } => {
                    let fenced = FencedCode {
                        content,
                        info: &info,
                        source: self.source,
                        value: &value,
                    };
                    let kind = E::code_block(&fenced).transpose()?;
                    Some(Block::CodeBlock(CodeBlock {
                        info,
                        kind,
                        span: token.span,
                        value,
                    }))
                }
                TokenKind::Heading { level, text } => {
                    let heading = self.heading(text, &token.span);
                    sections.open(usize::from(level), heading, token.span);
                    None
                }
                TokenKind::ItemStart => Some(Block::List(self.list(token.span)?)),
                TokenKind::Table(table) => Some(Block::Table(self.table(table, token.span))),
                TokenKind::Text => Some(Block::Paragraph(self.paragraph(token.span))),
            };
            if let Some(block) = found {
                sections.block(block);
            }
        }
        let read = sections.finish();
        let span = read
            .first()
            .zip(read.last())
            .map_or(at..at, |(first, last)| first.span.start..last.span.end);
        Ok(MarkdownBlock {
            sections: read,
            span,
        })
    }

    /// The marker of the next item of the list being read, past any blank
    /// lines before it.
    fn next_item(&mut self) -> Option<Range<usize>> {
        let blanks = self.tokens[self.pos..]
            .iter()
            .take_while(|token| matches!(token.kind, TokenKind::Blank))
            .count();
        let token = self.tokens.get(self.pos + blanks)?;
        if !matches!(token.kind, TokenKind::ItemStart) {
            return None;
        }
        let marker = token.span.clone();
        self.pos += blanks + 1;
        Some(marker)
    }

    fn paragraph(&mut self, first: Range<usize>) -> Paragraph {
        let start = first.start;
        let mut end = first.end;
        let mut lines = vec![line(self.source, first)];
        while let Some(token) = self.tokens.get(self.pos)
            && matches!(token.kind, TokenKind::Text)
        {
            end = token.span.end;
            lines.push(line(self.source, token.span.clone()));
            self.pos += 1;
        }
        Paragraph {
            lines,
            span: start..end,
        }
    }

    fn row(&self, row: RowToken) -> TableRow<E> {
        TableRow {
            cells: row.cells.into_iter().map(|cell| self.cell(cell)).collect(),
            span: row.span,
        }
    }

    /// Only the end of the enclosing list item, or of the tokens, ends a
    /// `MarkdownBlock`.
    fn stops(&self) -> bool {
        self.tokens
            .get(self.pos)
            .is_none_or(|token| matches!(token.kind, TokenKind::ItemEnd))
    }

    fn table(&self, table: TableToken, span: Range<usize>) -> Table<E> {
        Table {
            alignments: table.alignments,
            format: table.format,
            head: table.head.into_iter().map(|row| self.row(row)).collect(),
            rows: table.rows.into_iter().map(|row| self.row(row)).collect(),
            span,
        }
    }

    fn take(&mut self) -> Option<Token> {
        let token = self.tokens.get_mut(self.pos)?;
        self.pos += 1;
        Some(Token {
            kind: mem::replace(&mut token.kind, TokenKind::Blank),
            span: token.span.clone(),
        })
    }
}

impl<E> Sections<E>
where
    E: Extension,
{
    /// Adds `block` to the innermost open section. A block read while none
    /// is open opens the first section, with an empty heading.
    fn block(&mut self, block: Block<E>) {
        let span = block.span().clone();
        if self.open.is_empty() {
            self.open.push(untitled(span.start));
        }
        if let Some(section) = self.open.last_mut() {
            section.span.end = span.end;
            section.blocks.push(block);
        }
    }

    /// Puts `section`, just closed, into the section around it, or among the
    /// top-level sections.
    fn close(&mut self, section: Section<E>) {
        match self.open.last_mut() {
            Some(parent) => {
                parent.span.end = section.span.end;
                parent.subsections.push(section);
            }
            None => self.closed.push(section),
        }
    }

    /// Closes every open section and gives the top-level ones.
    fn finish(mut self) -> Vec<Section<E>> {
        while let Some(section) = self.open.pop() {
            self.close(section);
        }
        self.closed
    }

    const fn new() -> Self {
        Self {
            closed: Vec::new(),
            open: Vec::new(),
        }
    }

    /// Opens the section of a heading `level` marks deep: closes every open
    /// section at that depth or deeper, and fills a skipped level with a
    /// section whose heading is empty. `written` is the heading line.
    fn open(&mut self, level: usize, heading: Heading, written: Range<usize>) {
        while self.open.len() >= level
            && let Some(section) = self.open.pop()
        {
            self.close(section);
        }
        while self.open.len() + 1 < level {
            self.open.push(untitled(written.start));
        }
        self.open.push(Section {
            blocks: Vec::new(),
            heading,
            span: written,
            subsections: Vec::new(),
        });
    }
}

/// The body that starts at byte `start` of `source`, with code block
/// subtypes from `E`. It spans everything after the frontmatter.
pub fn parse<E>(source: &str, start: usize) -> Result<MarkdownBlock<E>, ParseError>
where
    E: Extension,
{
    let mut parser = Parser {
        extension: PhantomData,
        pos: 0,
        source,
        tokens: lex(source, start),
    };
    let body = parser.markdown_block(start)?;
    Ok(MarkdownBlock {
        sections: body.sections,
        span: start..source.len(),
    })
}

/// An empty heading at `at`: no children and an empty span.
const fn empty_heading(at: usize) -> Heading {
    Heading {
        children: Vec::new(),
        span: at..at,
    }
}

/// A section with an empty heading, starting at `at`.
const fn untitled<E>(at: usize) -> Section<E>
where
    E: Extension,
{
    Section {
        blocks: Vec::new(),
        heading: empty_heading(at),
        span: at..at,
        subsections: Vec::new(),
    }
}
