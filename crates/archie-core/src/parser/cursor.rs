//! A byte cursor over one region of the source.

use core::ops::Range;

use crate::parser::error::ParseError;

pub struct Cursor<'src> {
    /// The error of each value a reader dropped, in the order it met them.
    dropped: Vec<ParseError>,
    end: usize,
    pos: usize,
    source: &'src str,
}

impl<'src> Cursor<'src> {
    pub const fn at_end(&self) -> bool {
        self.pos >= self.end
    }

    pub const fn bump(&mut self) {
        if self.pos < self.end {
            self.pos += 1;
        }
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

    pub fn into_dropped(self) -> Vec<ParseError> {
        self.dropped
    }

    pub const fn new(source: &'src str, range: Range<usize>) -> Self {
        Self {
            dropped: Vec::new(),
            end: range.end,
            pos: range.start,
            source,
        }
    }

    pub fn note_drop(&mut self, error: ParseError) {
        self.dropped.push(error);
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
