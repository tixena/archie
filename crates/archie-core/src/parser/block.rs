//! Body tokens into blocks: sections, paragraphs, lists, tables and code
//! blocks.

use core::marker::PhantomData;
use core::mem;
use core::ops::Range;
use core::slice;

use crate::parser::ast::{
    Block, CodeBlock, Extension, FencedCode, Heading, Item, List, Paragraph, Section, Table,
    TableCell, TableRow,
};
use crate::parser::error::ParseError;
use crate::parser::inline::inlines;
use crate::parser::lexer::{Token, TokenKind, lex};
use crate::parser::table::{RowToken, TableToken};

struct Parser<'src, E> {
    extension: PhantomData<E>,
    pos: usize,
    source: &'src str,
    tokens: Vec<Token>,
}

impl<E> Parser<'_, E>
where
    E: Extension,
{
    /// The blocks up to the end of the enclosing list item, or up to a
    /// heading that closes a section of `level`.
    fn blocks(&mut self, level: u8) -> Result<Vec<Block<E>>, ParseError> {
        let mut blocks = Vec::new();
        while !self.stops(level) {
            let Some(token) = self.take() else {
                break;
            };
            match token.kind {
                TokenKind::Blank | TokenKind::ItemEnd => {}
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
                    blocks.push(Block::CodeBlock(CodeBlock {
                        info,
                        kind,
                        span: token.span,
                        value,
                    }));
                }
                TokenKind::Heading { level: found, text } => {
                    blocks.push(Block::Section(self.section(found, text, token.span)?));
                }
                TokenKind::ItemStart => blocks.push(Block::List(self.list(token.span)?)),
                TokenKind::Table(table) => blocks.push(Block::Table(self.table(table, token.span))),
                TokenKind::Text => blocks.push(Block::Paragraph(self.paragraph(token.span))),
            }
        }
        Ok(blocks)
    }

    /// A cell holds one paragraph of inline nodes, or nothing when empty.
    fn cell(&self, range: Range<usize>) -> TableCell<E> {
        let children = if range.is_empty() {
            Vec::new()
        } else {
            vec![Block::Paragraph(Paragraph {
                children: inlines(self.source, slice::from_ref(&range)),
                span: range.clone(),
            })]
        };
        TableCell {
            children,
            span: range,
        }
    }

    fn item(&mut self, marker: Range<usize>) -> Result<Item<E>, ParseError> {
        let children = self.blocks(0)?;
        if self
            .tokens
            .get(self.pos)
            .is_some_and(|token| matches!(token.kind, TokenKind::ItemEnd))
        {
            self.pos += 1;
        }
        let end = children.last().map_or(marker.end, |block| block.span().end);
        Ok(Item {
            children,
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
        let mut lines = vec![first];
        while let Some(token) = self.tokens.get(self.pos)
            && matches!(token.kind, TokenKind::Text)
        {
            end = token.span.end;
            lines.push(token.span.clone());
            self.pos += 1;
        }
        Paragraph {
            children: inlines(self.source, &lines),
            span: start..end,
        }
    }

    fn row(&self, row: RowToken) -> TableRow<E> {
        TableRow {
            cells: row.cells.into_iter().map(|cell| self.cell(cell)).collect(),
            span: row.span,
        }
    }

    fn section(
        &mut self,
        level: u8,
        text: Range<usize>,
        heading: Range<usize>,
    ) -> Result<Section<E>, ParseError> {
        let children = self.blocks(level)?;
        let end = children
            .last()
            .map_or(heading.end, |block| block.span().end);
        Ok(Section {
            children,
            heading: Heading {
                children: inlines(self.source, &[text]),
                span: heading.clone(),
            },
            level,
            span: heading.start..end,
        })
    }

    fn stops(&self, level: u8) -> bool {
        self.tokens
            .get(self.pos)
            .is_none_or(|token| match token.kind {
                TokenKind::ItemEnd => true,
                TokenKind::Heading {
                    level: found,
                    text: _,
                } => found <= level,
                TokenKind::Blank
                | TokenKind::CodeBlock {
                    content: _,
                    info: _,
                    value: _,
                }
                | TokenKind::ItemStart
                | TokenKind::Table(_)
                | TokenKind::Text => false,
            })
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

/// The blocks of the body that starts at byte `start` of `source`, with
/// code block subtypes from `E`.
pub fn parse<E>(source: &str, start: usize) -> Result<Vec<Block<E>>, ParseError>
where
    E: Extension,
{
    let mut parser = Parser {
        extension: PhantomData,
        pos: 0,
        source,
        tokens: lex(source, start),
    };
    parser.blocks(0)
}
