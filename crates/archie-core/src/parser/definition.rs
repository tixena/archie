//! Link reference definitions, such as `[guide]: <https://example.com> "Guide"`.

use core::cell::Cell;

/// A definition read from the start of a text.
pub struct Definition {
    /// The text between the brackets, as written.
    pub label: String,
    /// How many lines of the text the definition takes.
    pub lines: usize,
    /// The destination as written, without the angle brackets around it.
    pub target: String,
}

/// The definition a text starts with, if any, as far as the text goes.
pub struct Reading {
    pub definition: Option<Definition>,
    /// Whether a line added to the text could change `definition`.
    pub open: bool,
}

/// The bytes a definition is read from. Reading past their end marks the
/// reading open.
struct Bytes<'text> {
    bytes: &'text [u8],
    past_end: Cell<bool>,
}

impl Bytes<'_> {
    fn at(&self, at: usize) -> Option<u8> {
        let byte = self.bytes.get(at).copied();
        if byte.is_none() {
            self.past_end.set(true);
        }
        byte
    }

    /// Whether the backslash at `at` escapes the character after it, which
    /// it does for ASCII punctuation.
    fn escapes(&self, at: usize) -> bool {
        self.at(at + 1)
            .is_some_and(|byte| byte.is_ascii_punctuation())
    }
}

/// The definition that `text`, lines joined by `\n`, starts with: a label,
/// `:`, a destination and an optional title, with nothing after them on
/// their line.
pub fn read(text: &str) -> Reading {
    let bytes = Bytes {
        bytes: text.as_bytes(),
        past_end: Cell::new(false),
    };
    let definition = definition(text, &bytes);
    Reading {
        definition,
        open: bytes.past_end.get(),
    }
}

fn definition(text: &str, bytes: &Bytes<'_>) -> Option<Definition> {
    let close = label_end(text, bytes)?;
    if bytes.at(close + 1) != Some(b':') {
        return None;
    }
    let (target, after) = destination(text, bytes, gap(bytes, close + 2))?;
    let title = gap(bytes, after);
    let titled = if title > after {
        title_end(bytes, title).and_then(|title_close| line_end(bytes, title_close))
    } else {
        None
    };
    let end = titled.or_else(|| line_end(bytes, after))?;
    Some(Definition {
        label: text[1..close].to_owned(),
        lines: text[..end].matches('\n').count() + 1,
        target,
    })
}

/// The `]` that closes the label `text` opens: the first one not escaped,
/// with no unescaped `[` before it, around at most 999 characters that are
/// not all whitespace.
fn label_end(text: &str, bytes: &Bytes<'_>) -> Option<usize> {
    if bytes.at(0) != Some(b'[') {
        return None;
    }
    let mut at = 1;
    loop {
        match bytes.at(at)? {
            b'\\' if bytes.escapes(at) => at += 2,
            b'[' => return None,
            b']' => break,
            _ => at += 1,
        }
    }
    let inner = &text[1..at];
    (inner.chars().count() <= 999 && !inner.trim_ascii().is_empty()).then_some(at)
}

/// The destination at `at`, and where it ends: text between `<` and `>` on
/// one line, or a nonempty run with no space or control character whose
/// parentheses pair up.
fn destination(text: &str, bytes: &Bytes<'_>, at: usize) -> Option<(String, usize)> {
    if bytes.at(at) == Some(b'<') {
        let mut close = at + 1;
        loop {
            match bytes.at(close)? {
                b'\\' if bytes.escapes(close) => close += 2,
                b'>' => break,
                b'<' | b'\n' => return None,
                _ => close += 1,
            }
        }
        return Some((text[at + 1..close].to_owned(), close + 1));
    }
    let mut end = at;
    let mut depth = 0_usize;
    while let Some(byte) = bytes.at(end) {
        match byte {
            b'\\' if bytes.escapes(end) => end += 2,
            b'(' => {
                depth += 1;
                end += 1;
            }
            b')' if depth > 0 => {
                depth -= 1;
                end += 1;
            }
            b')' => break,
            _ if byte <= b' ' || byte == 0x7f => break,
            _ => end += 1,
        }
    }
    (end > at && depth == 0).then(|| (text[at..end].to_owned(), end))
}

/// Where the title at `at` ends, past its closing mark. A title is text
/// between `"`, `'` or parentheses, with no unescaped closing mark inside,
/// nor an unescaped `(` inside parentheses.
fn title_end(bytes: &Bytes<'_>, at: usize) -> Option<usize> {
    let close = match bytes.at(at)? {
        b'"' => b'"',
        b'\'' => b'\'',
        b'(' => b')',
        _ => return None,
    };
    let mut end = at + 1;
    loop {
        match bytes.at(end)? {
            b'\\' if bytes.escapes(end) => end += 2,
            byte if byte == close => return Some(end + 1),
            b'(' if close == b')' => return None,
            _ => end += 1,
        }
    }
}

/// Past the spaces and tabs at `at`, with at most one line ending among
/// them.
fn gap(bytes: &Bytes<'_>, at: usize) -> usize {
    let spaces = spaces_end(bytes, at);
    if bytes.at(spaces) == Some(b'\n') {
        spaces_end(bytes, spaces + 1)
    } else {
        spaces
    }
}

/// The end of the line at `at`, when only spaces and tabs follow `at` on it.
fn line_end(bytes: &Bytes<'_>, at: usize) -> Option<usize> {
    let end = spaces_end(bytes, at);
    matches!(bytes.at(end), None | Some(b'\n')).then_some(end)
}

/// Past the spaces and tabs at `at`.
fn spaces_end(bytes: &Bytes<'_>, at: usize) -> usize {
    let mut end = at;
    while matches!(bytes.at(end), Some(b' ' | b'\t')) {
        end += 1;
    }
    end
}
