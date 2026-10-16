//! Body tokens into blocks: sections, paragraphs, lists, tables and code
//! blocks.

use core::iter;
use core::marker::PhantomData;
use core::mem;
use core::ops::Range;

use crate::parser::ast::{
    Block, CodeBlock, Heading, Item, Kind, List, MarkdownBlock, Paragraph, Section, Table,
    TableCell, TableRow,
};
use crate::parser::extension::Extension;
use crate::parser::inline::line;
use crate::parser::lexer::{Marker, Token, TokenKind, lex, lex_lines};
use crate::parser::table::{Cell, RowToken, TableToken};

/// Where a markdown block has blank lines: between two of its blocks, or at
/// its end. A list is loose by them.
#[derive(Clone, Copy, Default)]
struct Blanks {
    between: bool,
    trailing: bool,
}

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
    /// A cell holds the markdown block its lines read as, one after another.
    fn cell(&self, cell: &Cell) -> TableCell<E> {
        let span = cell.span();
        let mut parser = Self {
            extension: PhantomData,
            pos: 0,
            source: self.source,
            tokens: lex_lines(self.source, &cell.lines),
        };
        let (body, _) = parser.markdown_block(span.start);
        TableCell {
            body,
            kind: Kind(None),
            span,
        }
    }

    /// The heading of the heading line at `written`, whose text is at `text`.
    /// A line with no text gives an empty heading at its start.
    fn heading(&self, text: Range<usize>, written: &Range<usize>) -> Heading<E> {
        if text.is_empty() {
            empty_heading(written.start)
        } else {
            Heading {
                children: line(self.source, text).children,
                kind: Kind(None),
                span: written.clone(),
            }
        }
    }

    fn item(&mut self, marker: Range<usize>) -> (Item<E>, Blanks) {
        let (body, blanks) = self.markdown_block(marker.end);
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
        let item = Item {
            body,
            kind: Kind(None),
            span: marker.start..end,
        };
        (item, blanks)
    }

    /// Consecutive items whose markers have the same bullet character, or
    /// the same delimiter, make one list; blank lines between them do not
    /// end it. Gives the list and whether it ends with a blank line.
    fn list(&mut self, marker: Marker, span: Range<usize>) -> (List<E>, bool) {
        let from = span.start;
        let (first, mut blanks) = self.item(span);
        let mut loose = blanks.between;
        let mut items = vec![first];
        while let Some((next, skipped)) = self.next_item(marker) {
            let (item, item_blanks) = self.item(next);
            loose |= blanks.trailing || skipped || item_blanks.between;
            blanks = item_blanks;
            items.push(item);
        }
        let end = items.last().map_or(from, |item| item.span.end);
        let list = List {
            items,
            kind: Kind(None),
            loose,
            span: from..end,
            start: marker.start(),
        };
        (list, blanks.trailing)
    }

    /// The sections up to the end of the enclosing list item, or of the
    /// tokens. It spans its sections, or is empty at `at` when it has none.
    fn markdown_block(&mut self, at: usize) -> (MarkdownBlock<E>, Blanks) {
        let mut sections = Sections::new();
        let mut blanks = Blanks::default();
        let mut read = false;
        while !self.stops() {
            let Some(token) = self.take() else {
                break;
            };
            let mut trailing = false;
            let found = match token.kind {
                TokenKind::Blank => {
                    blanks.trailing = true;
                    continue;
                }
                TokenKind::ItemEnd => continue,
                TokenKind::CodeBlock {
                    content,
                    info,
                    value,
                } => Some(Block::CodeBlock(CodeBlock {
                    content,
                    info,
                    kind: Kind(None),
                    span: token.span,
                    value,
                })),
                TokenKind::Heading { level, text } => {
                    let heading = self.heading(text, &token.span);
                    sections.open(usize::from(level), heading, token.span);
                    None
                }
                TokenKind::ItemStart(marker) => {
                    let (list, ends_blank) = self.list(marker, token.span);
                    trailing = ends_blank;
                    Some(Block::List(list))
                }
                TokenKind::Table(table) => Some(Block::Table(self.table(table, token.span))),
                TokenKind::Text => Some(Block::Paragraph(self.paragraph(token.span))),
            };
            blanks.between |= read && blanks.trailing;
            blanks.trailing = trailing;
            read = true;
            if let Some(block) = found {
                sections.block(block);
            }
        }
        let finished = sections.finish();
        let span = finished
            .first()
            .zip(finished.last())
            .map_or(at..at, |(first, last)| first.span.start..last.span.end);
        let block = MarkdownBlock {
            kind: Kind(None),
            sections: finished,
            span,
        };
        (block, blanks)
    }

    /// The marker of the next item of the list whose items carry `marker`,
    /// past any blank lines before it, and whether there were any.
    fn next_item(&mut self, marker: Marker) -> Option<(Range<usize>, bool)> {
        let blanks = self.tokens[self.pos..]
            .iter()
            .take_while(|token| matches!(token.kind, TokenKind::Blank))
            .count();
        let token = self.tokens.get(self.pos + blanks)?;
        let TokenKind::ItemStart(next) = token.kind else {
            return None;
        };
        if !marker.same_list(next) {
            return None;
        }
        let span = token.span.clone();
        self.pos += blanks + 1;
        Some((span, blanks > 0))
    }

    fn paragraph(&mut self, first: Range<usize>) -> Paragraph<E> {
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
            kind: Kind(None),
            lines,
            span: start..end,
        }
    }

    /// A row of `width` cells: a row with fewer gets empty cells at its end.
    fn row(&self, row: &RowToken, width: usize) -> TableRow<E> {
        let blank = Cell::line(row.span.end..row.span.end);
        let missing = width.saturating_sub(row.cells.len());
        TableRow {
            cells: row
                .cells
                .iter()
                .chain(iter::repeat_n(&blank, missing))
                .map(|cell| self.cell(cell))
                .collect(),
            kind: Kind(None),
            span: row.span.clone(),
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
        let width = table.alignments.len();
        Table {
            alignments: table.alignments,
            format: table.format,
            head: table.head.iter().map(|row| self.row(row, width)).collect(),
            kind: Kind(None),
            rows: table.rows.iter().map(|row| self.row(row, width)).collect(),
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
    fn open(&mut self, level: usize, heading: Heading<E>, written: Range<usize>) {
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
            kind: Kind(None),
            span: written,
            subsections: Vec::new(),
        });
    }
}

/// The markdown block that starts at byte `start` of `source`, with no node
/// claimed yet. It spans everything from `start` to the end of `source`.
pub fn parse<E>(source: &str, start: usize) -> MarkdownBlock<E>
where
    E: Extension,
{
    let mut parser = Parser {
        extension: PhantomData,
        pos: 0,
        source,
        tokens: lex(source, start),
    };
    let (body, _) = parser.markdown_block(start);
    MarkdownBlock {
        kind: Kind(None),
        sections: body.sections,
        span: start..source.len(),
    }
}

/// An empty heading at `at`: no children and an empty span.
const fn empty_heading<E>(at: usize) -> Heading<E>
where
    E: Extension,
{
    Heading {
        children: Vec::new(),
        kind: Kind(None),
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
        kind: Kind(None),
        span: at..at,
        subsections: Vec::new(),
    }
}
