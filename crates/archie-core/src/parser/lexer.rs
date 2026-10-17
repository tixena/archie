//! Body lines into tokens. The indentation that says which list item a line
//! belongs to becomes explicit item starts and ends, and a code block, a block
//! quote, an HTML block and a link definition are each read whole, so the
//! grammar over the tokens is context-free.

use core::ops::Range;

use crate::parser::definition::{self, Target};
use crate::parser::html::{self, End};
use crate::parser::line::{line_at, line_from};
use crate::parser::table::{self, TableToken};

/// How deep list items, markdown blocks and table cells nest, counted
/// together: 43 grid tables, each in the cell of the one before, below the
/// headings `#` to `######`, around emphasis 1000 deep, overflow a 2 MB debug
/// stack when the tree is cloned.
pub const BLOCK_CAP: usize = 40;

/// The marker that starts a list item.
#[derive(Clone, Copy)]
pub enum Marker {
    /// `-`, `*` or `+`.
    Bullet(char),
    /// `1.` or `1)`: the number and the delimiter.
    Ordered { delimiter: char, number: u64 },
}

impl Marker {
    /// Whether an item with this marker may start a list on a line that
    /// would otherwise continue a paragraph: a bullet, or an ordered marker
    /// numbered 1.
    const fn interrupts(self) -> bool {
        match self {
            Self::Bullet(_) => true,
            Self::Ordered {
                delimiter: _,
                number,
            } => number == 1,
        }
    }

    /// Whether an item with `other` belongs to the list this marker is in:
    /// the same bullet character, or the same delimiter.
    pub const fn same_list(self, other: Self) -> bool {
        self.symbol() == other.symbol()
    }

    /// The first number of the list this marker starts, `None` for a bullet.
    pub const fn start(self) -> Option<u64> {
        match self {
            Self::Bullet(_) => None,
            Self::Ordered {
                delimiter: _,
                number,
            } => Some(number),
        }
    }

    /// The bullet character, or the delimiter of an ordered marker.
    const fn symbol(self) -> char {
        match self {
            Self::Bullet(bullet) => bullet,
            Self::Ordered {
                delimiter,
                number: _,
            } => delimiter,
        }
    }
}

pub struct Token {
    pub kind: TokenKind,
    pub span: Range<usize>,
}

pub enum TokenKind {
    Blank,
    BlockQuote {
        value: String,
    },
    /// A code block: the range of its lines, of its info string, trimmed,
    /// and its lines as the block holds them.
    CodeBlock {
        content: Range<usize>,
        info: Range<usize>,
        value: String,
    },
    /// A link reference definition: its label as written and its target.
    Definition {
        label: String,
        target: Target,
    },
    Heading {
        level: u8,
        text: Range<usize>,
    },
    Html {
        value: String,
    },
    ItemEnd,
    ItemStart(Marker),
    /// A table, and how many containers hold it.
    Table {
        depth: usize,
        table: TableToken,
    },
    Text,
    ThematicBreak,
    /// `===` or `---` under text: the level of the heading it makes.
    Underline(u8),
}

/// The block a line starts, other than a paragraph.
enum Start {
    Break,
    Fence {
        length: usize,
        mark: char,
    },
    Heading {
        level: u8,
        text: Range<usize>,
    },
    Html(html::Start),
    Item {
        marker: Marker,
        width: usize,
    },
    Quote,
    Table {
        next: usize,
        span: Range<usize>,
        table: TableToken,
    },
    Underline(u8),
}

struct Lexer<'src> {
    /// How many leading spaces of each line are left out, at most.
    base: usize,
    /// The content column of each open list item, outermost first.
    levels: Vec<usize>,
    /// How many containers hold the text read: list items, markdown blocks
    /// and table cells.
    outer: usize,
    pos: usize,
    source: &'src str,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    /// Reads a block quote whose first `>` is at `at`: that line, then each
    /// line whose text starts with `>` within three columns of the
    /// enclosing item's text. Each line loses its `>` and one space after it.
    fn block_quote(&mut self, at: usize) {
        let source = self.source;
        let level = self.level();
        let first = line_at(source, at);
        let mut lines = vec![unquote(&source[at..first.end])];
        let mut end = first.end;
        while self.pos < source.len() {
            let line = line_from(source, self.pos, self.base);
            let text = &source[line.start..line.end];
            let content = text.trim_start_matches(' ');
            let indent = text.len() - content.len();
            if indent < level || indent >= level + 4 || !content.starts_with('>') {
                break;
            }
            lines.push(unquote(content));
            end = line.end;
            self.pos = line.next;
        }
        let value = lines.join("\n");
        self.push(TokenKind::BlockQuote { value }, at..end);
    }

    /// Ends every open item whose content column is to the right of `column`.
    fn close_items(&mut self, column: usize, at: usize) {
        while self.levels.last().is_some_and(|&level| level > column) {
            self.levels.pop();
            self.push(TokenKind::ItemEnd, at..at);
        }
    }

    /// Reads the lines after an opening fence of `length` `mark`s, up to the
    /// closing fence, the end of the enclosing list item, or the end of the
    /// file.
    fn code_block(&mut self, column: usize, at: usize, end: usize, length: usize, mark: char) {
        let source = self.source;
        let after = &source[at + length..end];
        let from = at + length + (after.len() - after.trim_start().len());
        let info = from..from + after.trim().len();
        let level = self.level();
        let mut lines: Vec<&str> = Vec::new();
        let mut block_end = end;
        let content_start = self.pos;
        let mut content_end = content_start;
        while self.pos < source.len() {
            let line = line_from(source, self.pos, self.base);
            let text = &source[line.start..line.end];
            let content = text.trim_start_matches(' ');
            let indent = text.len() - content.len();
            let trimmed = trim_end_spaces(content);
            if !trimmed.is_empty() && indent < level {
                break;
            }
            self.pos = line.next;
            if trimmed.len() >= length && trimmed.chars().all(|character| character == mark) {
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
    /// `line_start`. `continues` is set when the line would continue a
    /// paragraph.
    fn content(&mut self, line_start: usize, at: usize, end: usize, continues: bool) {
        let column = at - line_start;
        let level = self.level();
        if column >= level + 4 {
            if continues {
                self.push(TokenKind::Text, at..end);
            } else {
                self.indented_code(line_start, level + 4);
            }
            return;
        }
        match self.start(line_start, at..end, self.pos, continues) {
            Some(Start::Break) => self.push(TokenKind::ThematicBreak, at..end),
            Some(Start::Fence { length, mark }) => self.code_block(column, at, end, length, mark),
            Some(Start::Heading {
                level: heading_level,
                text,
            }) => {
                let kind = TokenKind::Heading {
                    level: heading_level,
                    text: at + text.start..at + text.end,
                };
                self.push(kind, at..end);
            }
            Some(Start::Html(start)) => self.html(line_start, at, start),
            Some(Start::Item { marker, width }) => {
                let after = &self.source[at + width..end];
                let inner = after.trim_start_matches(' ');
                let spaces = after.len() - inner.len();
                // One to four spaces set the content column; none, or five or
                // more, put it one space past the marker.
                let gap = if spaces == 0 || spaces > 4 { 1 } else { spaces };
                self.push(TokenKind::ItemStart(marker), at..at + width);
                self.levels.push(column + width + gap);
                if !inner.is_empty() {
                    self.content(line_start, end - inner.len(), end, false);
                }
            }
            Some(Start::Quote) => self.block_quote(at),
            Some(Start::Table { next, span, table }) => {
                self.pos = next;
                let depth = self.depth();
                self.push(TokenKind::Table { depth, table }, span);
            }
            Some(Start::Underline(heading_level)) => {
                self.push(TokenKind::Underline(heading_level), at..end);
            }
            None => {
                if continues || !self.definition(at, end) {
                    self.push(TokenKind::Text, at..end);
                }
            }
        }
    }

    /// The line at `at`, from its text to its end, and where the next line
    /// starts, when it would continue a paragraph.
    fn continuation(&self, at: usize) -> Option<(Range<usize>, usize)> {
        let source = self.source;
        let level = self.level();
        let line = line_from(source, at, self.base);
        let content = source[line.start..line.end].trim_start_matches(' ');
        let from = line.end - content.len();
        let text = from..from + trim_end_spaces(content).len();
        let indent = from - line.start;
        let continues = !text.is_empty()
            && (indent >= level + 4
                || (indent >= level
                    && self
                        .start(line.start, text.clone(), line.next, true)
                        .is_none()));
        continues.then_some((text, line.next))
    }

    /// Reads a link reference definition that starts at `at`, over its line
    /// and as many lines after it as it needs that would continue a
    /// paragraph, each up to its line ending, so a title over a line end
    /// keeps the spaces and tabs before it. Gives whether there was one.
    fn definition(&mut self, at: usize, end: usize) -> bool {
        let source = self.source;
        if !source[at..end].starts_with('[') {
            return false;
        }
        let written = |from: usize| &source[from..line_at(source, from).end];
        let mut text = written(at).to_owned();
        let mut ends = vec![(end, self.pos)];
        let read = loop {
            let reading = definition::read(&text);
            let next = ends.last().map_or(self.pos, |&(_, next)| next);
            let more = if reading.open && next < source.len() {
                self.continuation(next)
            } else {
                None
            };
            let Some((line, after)) = more else {
                break reading.definition;
            };
            text.push('\n');
            text.push_str(written(line.start));
            ends.push((line.end, after));
        };
        let Some(found) = read else {
            return false;
        };
        let (last, next) = ends[found.lines - 1];
        self.pos = next;
        let kind = TokenKind::Definition {
            label: found.label,
            target: found.target,
        };
        self.push(kind, at..last);
        true
    }

    /// How many containers hold the line read now.
    const fn depth(&self) -> usize {
        self.outer + self.levels.len()
    }

    /// Reads an HTML block whose first line starts at `line_start` and whose
    /// text starts at `at`, up to the line that ends it by `start`, the end
    /// of the enclosing list item, or the end of the file. Its lines are kept
    /// as written, past the item's text column.
    fn html(&mut self, line_start: usize, at: usize, start: html::Start) {
        let source = self.source;
        let level = self.level();
        let first = line_at(source, at);
        let from = line_start + level;
        let mut lines = vec![&source[from..first.end]];
        let mut block_end = first.end;
        let mut blanks = Vec::new();
        let mut scan = self.pos;
        let mut open = !start.end.closes(&source[at + start.opening..first.end]);
        while open && scan < source.len() {
            let line = line_from(source, scan, self.base);
            let text = &source[line.start..line.end];
            let content = text.trim_start_matches(' ');
            let indent = text.len() - content.len();
            if trim_end_spaces(content).is_empty() {
                if matches!(start.end, End::Blank) {
                    break;
                }
                blanks.push(&text[indent.min(level)..]);
            } else if indent < level {
                break;
            } else {
                lines.append(&mut blanks);
                lines.push(&text[level..]);
                block_end = line.end;
                self.pos = line.next;
                open = !start.end.closes(text);
            }
            scan = line.next;
        }
        let value = lines.join("\n");
        self.push(TokenKind::Html { value }, from..block_end);
    }

    /// Reads an indented code block whose first line starts at `line_start`
    /// and whose text starts at `column`: that line, then every line
    /// indented at least as far, with the blank lines between them.
    fn indented_code(&mut self, line_start: usize, column: usize) {
        let source = self.source;
        let first = line_at(source, line_start);
        let start = line_start + column;
        let mut lines = vec![&source[start..first.end]];
        let mut block_end = first.end;
        let mut blanks = Vec::new();
        let mut scan = self.pos;
        while scan < source.len() {
            let line = line_from(source, scan, self.base);
            let text = &source[line.start..line.end];
            let content = text.trim_start_matches(' ');
            let indent = text.len() - content.len();
            if trim_end_spaces(content).is_empty() {
                blanks.push(&text[indent.min(column)..]);
            } else if indent >= column {
                lines.append(&mut blanks);
                lines.push(&text[column..]);
                block_end = line.end;
                self.pos = line.next;
            } else {
                break;
            }
            scan = line.next;
        }
        let kind = TokenKind::CodeBlock {
            content: start..block_end,
            info: start..start,
            value: lines.join("\n"),
        };
        self.push(kind, start..block_end);
    }

    /// The content column of the innermost open item, or zero.
    fn level(&self) -> usize {
        self.levels.last().copied().unwrap_or(0)
    }

    fn line(&mut self) {
        let line = line_from(self.source, self.pos, self.base);
        self.pos = line.next;
        let text = &self.source[line.start..line.end];
        let content = text.trim_start_matches(' ');
        let trimmed = trim_end_spaces(content);
        if trimmed.is_empty() {
            // An item whose marker line is empty ends at a blank line after it.
            if self
                .tokens
                .last()
                .is_some_and(|token| matches!(token.kind, TokenKind::ItemStart(_)))
            {
                self.levels.pop();
                self.push(TokenKind::ItemEnd, line.start..line.start);
            }
            self.push(TokenKind::Blank, line.start..line.start);
            return;
        }
        let at = line.start + (text.len() - content.len());
        let open = self.levels.len();
        let after_text = self
            .tokens
            .last()
            .is_some_and(|token| matches!(token.kind, TokenKind::Text));
        self.close_items(at - line.start, line.start);
        let continues = after_text && self.levels.len() == open;
        self.content(line.start, at, at + trimmed.len(), continues);
    }

    fn push(&mut self, kind: TokenKind, span: Range<usize>) {
        self.tokens.push(Token { kind, span });
    }

    /// The block that the text at `text`, on the line starting at
    /// `line_start`, starts, read on from `next`, tried in the order of
    /// precedence `CommonMark` gives them. `continues` is set when the line
    /// would continue a paragraph, which only some blocks may interrupt. An
    /// item or a table whose content would lie deeper than `BLOCK_CAP` is
    /// not opened, and its line is text.
    fn start(
        &self,
        line_start: usize,
        text: Range<usize>,
        next: usize,
        continues: bool,
    ) -> Option<Start> {
        let rest = &self.source[text.clone()];
        if continues && let Some(level) = underline(rest) {
            return Some(Start::Underline(level));
        }
        if let Some((mark, length)) = fence(rest) {
            return Some(Start::Fence { length, mark });
        }
        if is_thematic_break(rest) {
            return Some(Start::Break);
        }
        if let Some((level, title)) = heading(rest) {
            return Some(Start::Heading { level, text: title });
        }
        if rest.starts_with('>') {
            return Some(Start::Quote);
        }
        if let Some(start) = html::start(rest).filter(|start| !continues || start.interrupts) {
            return Some(Start::Html(start));
        }
        let opens = self.depth() < BLOCK_CAP;
        if let Some((marker, width)) = list_marker(rest).filter(|&(marker, width)| {
            // An empty item never interrupts a paragraph.
            !continues || (marker.interrupts() && !rest[width..].trim_start_matches(' ').is_empty())
        }) {
            return opens.then_some(Start::Item { marker, width });
        }
        if !opens {
            return None;
        }
        table::read(self.source, line_start, text, next, self.level(), self.base).map(
            |(table, span, after)| Start::Table {
                next: after,
                span,
                table,
            },
        )
    }
}

/// The tokens of `source` from byte `start`, each line read with at most
/// `base` of its leading spaces left out, so its columns count from `base`.
/// `outer` containers hold the text.
pub fn lex_from(source: &str, start: usize, base: usize, outer: usize) -> Vec<Token> {
    let mut lexer = Lexer {
        base,
        levels: Vec::new(),
        outer,
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

/// The level of an ATX heading and the range of its text: from the first
/// character after the spaces that follow the `#`s, up to the spaces and
/// tabs before the closing sequence, a final run of `#`s that a space or a
/// tab precedes or that is the whole text.
fn heading(rest: &str) -> Option<(u8, Range<usize>)> {
    let text = rest.trim_start_matches('#');
    let level = u8::try_from(rest.len() - text.len())
        .ok()
        .filter(|level| (1..=6).contains(level))?;
    if !text.is_empty() && !text.starts_with(' ') {
        return None;
    }
    let title = text.trim_start_matches(' ');
    let start = rest.len() - title.len();
    let before_run = title.trim_end_matches('#');
    let end = if before_run.is_empty() || before_run.ends_with([' ', '\t']) {
        start + trim_end_spaces(before_run).len()
    } else {
        rest.len()
    };
    Some((level, start..end))
}

/// The character and the length of an opening fence: three or more
/// backticks, whose info string holds no backtick, or three or more tildes.
fn fence(rest: &str) -> Option<(char, usize)> {
    let mark = rest
        .chars()
        .next()
        .filter(|mark| matches!(mark, '`' | '~'))?;
    let info = rest.trim_start_matches(mark);
    let length = rest.len() - info.len();
    (length >= 3 && (mark == '~' || !info.contains('`'))).then_some((mark, length))
}

/// The list marker `rest` starts with, and its width: a bullet, or one to
/// nine digits and a `.` or `)`, followed by a space or by the end of the
/// line.
fn list_marker(rest: &str) -> Option<(Marker, usize)> {
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (marker, width) = match digits {
        0 => {
            let bullet = rest
                .chars()
                .next()
                .filter(|bullet| matches!(bullet, '-' | '*' | '+'))?;
            (Marker::Bullet(bullet), 1)
        }
        1..=9 => {
            let delimiter = rest[digits..]
                .chars()
                .next()
                .filter(|delimiter| matches!(delimiter, '.' | ')'))?;
            let number = rest[..digits].parse().ok()?;
            (Marker::Ordered { delimiter, number }, digits + 1)
        }
        _ => return None,
    };
    matches!(rest[width..].chars().next(), None | Some(' ')).then_some((marker, width))
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

/// The level of the heading that `rest`, a run of `=` or of `-` under
/// paragraph text, underlines: 1 for `=`, 2 for `-`.
fn underline(rest: &str) -> Option<u8> {
    let mark = rest.chars().next()?;
    let level = match mark {
        '=' => 1,
        '-' => 2,
        _ => return None,
    };
    rest.chars()
        .all(|character| character == mark)
        .then_some(level)
}

/// `text` without the spaces and tabs at its end: a line holding nothing
/// else is blank.
pub fn trim_end_spaces(text: &str) -> &str {
    text.trim_end_matches([' ', '\t'])
}

/// A quoted line without its `>` and one space after it.
fn unquote(line: &str) -> &str {
    let rest = line.strip_prefix('>').unwrap_or(line);
    rest.strip_prefix(' ').unwrap_or(rest)
}

/// Reads `lines`, ranges of `source`, as consecutive lines of one text, each
/// with at most `base` of its leading spaces left out, the way `lex_from`
/// reads a file, held by `outer` containers. Spans count from the start of
/// `source`.
pub fn lex_lines(source: &str, lines: &[Range<usize>], base: usize, outer: usize) -> Vec<Token> {
    let mut starts = Vec::with_capacity(lines.len());
    let mut text = String::new();
    for line in lines {
        if !starts.is_empty() {
            text.push('\n');
        }
        starts.push(text.len());
        text.push_str(&source[line.clone()]);
    }
    let place = |at: usize| {
        let index = starts
            .partition_point(|&start| start <= at)
            .saturating_sub(1);
        lines
            .get(index)
            .zip(starts.get(index))
            .map_or(at, |(line, &start)| line.start + (at - start))
    };
    let mut tokens = lex_from(&text, 0, base, outer);
    for token in &mut tokens {
        token.span = place(token.span.start)..place(token.span.end);
        match &mut token.kind {
            TokenKind::CodeBlock {
                content,
                info,
                value: _,
            } => {
                *content = place(content.start)..place(content.end);
                *info = place(info.start)..place(info.end);
            }
            TokenKind::Heading {
                level: _,
                text: range,
            } => *range = place(range.start)..place(range.end),
            TokenKind::Table { depth: _, table } => table.place(&place),
            TokenKind::Blank
            | TokenKind::BlockQuote { value: _ }
            | TokenKind::Definition {
                label: _,
                target: _,
            }
            | TokenKind::Html { value: _ }
            | TokenKind::ItemEnd
            | TokenKind::ItemStart(_)
            | TokenKind::Text
            | TokenKind::ThematicBreak
            | TokenKind::Underline(_) => {}
        }
    }
    tokens
}
