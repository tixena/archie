//! A byte cursor over one region of the source.

use core::ops::Range;

use crate::parser::error::ParseError;

/// An entry or a key a reader dropped: the text it covers, and the error
/// that dropped it.
pub struct Dropped {
    pub error: ParseError,
    pub span: Range<usize>,
}

/// Where an entry starts: the cursor's position, and how many drops were
/// noted before it.
#[derive(Clone, Copy)]
pub struct Mark {
    pub noted: usize,
    pub pos: usize,
}

/// A byte cursor, with the `state` a reader keeps while it reads.
pub struct Cursor<'src, S = ()> {
    /// Each entry and key a reader dropped, in the order it met them.
    dropped: Vec<Dropped>,
    end: usize,
    pos: usize,
    source: &'src str,
    state: S,
}

impl<'src, S> Cursor<'src, S> {
    pub const fn at_end(&self) -> bool {
        self.pos >= self.end
    }

    pub const fn bump(&mut self) {
        if self.pos < self.end {
            self.pos += 1;
        }
    }

    /// Notes the drop of the entry from `mark` to the cursor, with `error`, in
    /// place of the drops noted inside it. Its text ends with its last line
    /// that holds more than white space or a comment.
    pub fn drop_since(&mut self, mark: Mark, error: ParseError) {
        self.forget_since(mark);
        let mut end = mark.pos;
        let mut at = mark.pos;
        for (number, line) in self.source[mark.pos..self.pos]
            .split_inclusive('\n')
            .enumerate()
        {
            let text = line.trim_end_matches([' ', '\t', '\r', '\n']);
            let content = text.trim_start_matches([' ', '\t']);
            if number == 0 || !(content.is_empty() || content.starts_with('#')) {
                end = at + text.len();
            }
            at += line.len();
        }
        let span = mark.pos..end;
        self.dropped.push(Dropped { error, span });
    }

    pub fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.pos += 1;
        }
        found
    }

    /// Advances past `text` when the cursor is at it.
    pub fn eat_str(&mut self, text: &str) -> bool {
        let found = self.rest().starts_with(text);
        if found {
            self.pos += text.len();
        }
        found
    }

    pub const fn end(&self) -> usize {
        self.end
    }

    pub fn error<T, E>(&self, message: &str) -> Result<T, E>
    where
        E: From<ParseError>,
    {
        Err(ParseError::new(message, self.pos..self.pos).into())
    }

    /// Forgets the drops noted since `mark`.
    pub fn forget_since(&mut self, mark: Mark) {
        self.dropped.truncate(mark.noted);
    }

    /// The character written as `digits` hexadecimal digits at the cursor.
    pub fn hex_char(&mut self, digits: usize) -> Option<char> {
        let hex = self.rest().get(..digits)?;
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let character = char::from_u32(u32::from_str_radix(hex, 16).ok()?)?;
        self.pos += digits;
        Some(character)
    }

    pub fn into_dropped(self) -> Vec<Dropped> {
        self.dropped
    }

    pub const fn mark(&self) -> Mark {
        Mark {
            noted: self.dropped.len(),
            pos: self.pos,
        }
    }

    pub const fn new(source: &'src str, range: Range<usize>, state: S) -> Self {
        Self {
            dropped: Vec::new(),
            end: range.end,
            pos: range.start,
            source,
            state,
        }
    }

    pub fn peek(&self) -> Option<u8> {
        self.peek_at(0)
    }

    pub fn peek_at(&self, offset: usize) -> Option<u8> {
        let at = self.pos + offset;
        (at < self.end).then(|| self.source.as_bytes()[at])
    }

    pub const fn pos(&self) -> usize {
        self.pos
    }

    pub fn rest(&self) -> &'src str {
        &self.source[self.pos..self.end]
    }

    /// The rest of the current line, without its line ending.
    pub fn rest_of_line(&self) -> &'src str {
        let rest = self.rest();
        let line = rest.find('\n').map_or(rest, |at| &rest[..at]);
        line.strip_suffix('\r').unwrap_or(line)
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos.min(self.end);
    }

    pub fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
    }

    pub fn slice(&self, range: Range<usize>) -> &'src str {
        &self.source[range]
    }

    pub const fn state(&self) -> &S {
        &self.state
    }

    pub const fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }

    /// Advances to the first of `stops`, or to the end, and returns what it
    /// passed over.
    pub fn take_until(&mut self, stops: &[u8]) -> &'src str {
        let start = self.pos;
        while self.peek().is_some_and(|byte| !stops.contains(&byte)) {
            self.pos += 1;
        }
        &self.source[start..self.pos]
    }
}
