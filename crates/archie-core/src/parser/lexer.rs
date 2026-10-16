//! Body lines into tokens. The indentation that says which list item a line
//! belongs to becomes explicit item starts and ends, and a fenced code block
//! is read whole, so the grammar over the tokens is context-free.

use core::ops::Range;

use crate::parser::line::line_at;
use crate::parser::table::{self, TableToken};

pub struct Token {
    pub kind: TokenKind,
    pub span: Range<usize>,
}

pub enum TokenKind {
    Blank,
    CodeBlock {
        content: Range<usize>,
        info: String,
        value: String,
    },
    Heading {
        level: u8,
        text: Range<usize>,
    },
    ItemEnd,
    ItemStart,
    Table(TableToken),
    Text,
}

struct Lexer<'src> {
    /// The content column of each open list item, outermost first.
    levels: Vec<usize>,
    pos: usize,
    source: &'src str,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    /// Ends every open item whose content column is to the right of `column`.
    fn close_items(&mut self, column: usize, at: usize) {
        while self.levels.last().is_some_and(|&level| level > column) {
            self.levels.pop();
            self.push(TokenKind::ItemEnd, at..at);
        }
    }

    /// Reads the lines after an opening fence, up to the closing fence, the
    /// end of the enclosing list item, or the end of the file.
    fn code_block(&mut self, column: usize, at: usize, end: usize, ticks: usize) {
        let source = self.source;
        let info = source[at + ticks..end].trim().to_owned();
        let level = self.level();
        let mut lines: Vec<&str> = Vec::new();
        let mut block_end = end;
        let content_start = self.pos;
        let mut content_end = content_start;
        while self.pos < source.len() {
            let line = line_at(source, self.pos);
            let text = &source[line.start..line.end];
            let content = text.trim_start_matches(' ');
            let indent = text.len() - content.len();
            let trimmed = content.trim_end();
            if !trimmed.is_empty() && indent < level {
                break;
            }
            self.pos = line.next;
            if trimmed.len() >= ticks && trimmed.bytes().all(|byte| byte == b'`') {
                block_end = line.start + indent + trimmed.len();
                break;
            }
            lines.push(&text[indent.min(column)..]);
            block_end = line.end;
            content_end = line.end;
        }
        let value = lines.join("\n");
        let content = content_start..content_end;
        self.push(
            TokenKind::CodeBlock {
                content,
                info,
                value,
            },
            at..block_end,
        );
    }

    /// Classifies what is written from `at` to `end` on the line starting at
    /// `line_start`.
    fn content(&mut self, line_start: usize, at: usize, end: usize) {
        let rest = &self.source[at..end];
        if let Some((level, text_at)) = heading(rest) {
            let text = at + text_at..end;
            self.push(TokenKind::Heading { level, text }, at..end);
        } else if let Some(ticks) = fence(rest) {
            self.code_block(at - line_start, at, end, ticks);
        } else if is_bullet(rest) {
            self.push(TokenKind::ItemStart, at..at + 1);
            self.levels.push(at - line_start + 2);
            let after = &rest[1..];
            let inner = after.trim_start_matches(' ');
            if !inner.is_empty() {
                self.content(line_start, end - inner.len(), end);
            }
        } else if let Some((table, span, next)) =
            table::read(self.source, line_start, at..end, self.pos, self.level())
        {
            self.pos = next;
            self.push(TokenKind::Table(table), span);
        } else {
            self.push(TokenKind::Text, at..end);
        }
    }

    /// The content column of the innermost open item, or zero.
    fn level(&self) -> usize {
        self.levels.last().copied().unwrap_or(0)
    }

    fn line(&mut self) {
        let line = line_at(self.source, self.pos);
        self.pos = line.next;
        let text = &self.source[line.start..line.end];
        let content = text.trim_start_matches(' ');
        let trimmed = content.trim_end();
        if trimmed.is_empty() {
            self.push(TokenKind::Blank, line.start..line.start);
            return;
        }
        let at = line.start + (text.len() - content.len());
        self.close_items(at - line.start, line.start);
        self.content(line.start, at, at + trimmed.len());
    }

    fn push(&mut self, kind: TokenKind, span: Range<usize>) {
        self.tokens.push(Token { kind, span });
    }
}

pub fn lex(source: &str, start: usize) -> Vec<Token> {
    let mut lexer = Lexer {
        levels: Vec::new(),
        pos: start,
        source,
        tokens: Vec::new(),
    };
    while lexer.pos < source.len() {
        lexer.line();
    }
    lexer.close_items(0, source.len());
    lexer.tokens
}

/// The level of an ATX heading and the offset its text starts at.
fn heading(rest: &str) -> Option<(u8, usize)> {
    let text = rest.trim_start_matches('#');
    let level = u8::try_from(rest.len() - text.len())
        .ok()
        .filter(|level| (1..=6).contains(level))?;
    if !text.is_empty() && !text.starts_with(' ') {
        return None;
    }
    let title = text.trim_start_matches(' ');
    Some((level, rest.len() - title.len()))
}

/// The number of backticks of an opening fence.
fn fence(rest: &str) -> Option<usize> {
    let info = rest.trim_start_matches('`');
    let ticks = rest.len() - info.len();
    (ticks >= 3 && !info.contains('`')).then_some(ticks)
}

fn is_bullet(rest: &str) -> bool {
    let mut characters = rest.chars();
    matches!(characters.next(), Some('-' | '*' | '+'))
        && matches!(characters.next(), None | Some(' '))
        && !is_thematic_break(rest)
}

/// Three or more of the same `-`, `*` or `_`, with nothing but spaces
/// between them.
fn is_thematic_break(rest: &str) -> bool {
    let Some(mark) = rest.chars().next().filter(|mark| "-*_".contains(*mark)) else {
        return false;
    };
    rest.chars()
        .all(|character| character == mark || character == ' ')
        && rest.chars().filter(|character| *character == mark).count() >= 3
}
