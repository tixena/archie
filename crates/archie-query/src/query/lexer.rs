//! Splits a query into tokens.

use core::ops::Range;

use crate::error::QueryError;

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ge,
    Gt,
    Has,
    Le,
    Lt,
    Ne,
}

/// What a token is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Dot,
    /// A bare word: `fm`, a key, `and`, `or`, `not`, `true`, `false`.
    Ident(String),
    LParen,
    Op(CmpOp),
    RParen,
    /// A double-quoted string, unescaped.
    Str(String),
    /// A number or a date, as written.
    Word(String),
}

/// One token and where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub span: Range<usize>,
}

/// Splits `text` into tokens.
pub fn lex(text: &str) -> Result<Vec<Token>, QueryError> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let start = at;
        let kind = match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                at += 1;
                continue;
            }
            b'(' => {
                at += 1;
                Kind::LParen
            }
            b')' => {
                at += 1;
                Kind::RParen
            }
            b'.' => {
                // A key after a dot is read with ident rules: `fm.2fa`, `fm.has`.
                let key_end = run(bytes, start + 1, is_ident_byte);
                tokens.push(Token {
                    kind: Kind::Dot,
                    span: start..start + 1,
                });
                if key_end > start + 1 {
                    tokens.push(Token {
                        kind: Kind::Ident(text[start + 1..key_end].to_owned()),
                        span: start + 1..key_end,
                    });
                }
                at = key_end;
                continue;
            }
            b'"' => {
                let (value, end) = string(text, start)?;
                at = end;
                Kind::Str(value)
            }
            b'=' | b'!' | b'<' | b'>' => {
                let (op, width) = operator(&bytes[start..]).ok_or_else(|| {
                    QueryError::new(
                        "expected `==`, `!=`, `<`, `<=`, `>` or `>=`",
                        start..start + 1,
                    )
                })?;
                at += width;
                Kind::Op(op)
            }
            b'0'..=b'9' | b'-' => {
                at = run(bytes, start, is_word_byte);
                Kind::Word(text[start..at].to_owned())
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                at = run(bytes, start, is_ident_byte);
                let word = &text[start..at];
                if word == "has" {
                    Kind::Op(CmpOp::Has)
                } else {
                    Kind::Ident(word.to_owned())
                }
            }
            _ => {
                let width = text[start..].chars().next().map_or(1, char::len_utf8);
                return Err(QueryError::new(
                    "unexpected character",
                    start..start + width,
                ));
            }
        };
        tokens.push(Token {
            kind,
            span: start..at,
        });
    }
    Ok(tokens)
}

const fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

const fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'+' | b':' | b'.')
}

const fn operator(rest: &[u8]) -> Option<(CmpOp, usize)> {
    match rest {
        [b'=', b'=', ..] => Some((CmpOp::Eq, 2)),
        [b'!', b'=', ..] => Some((CmpOp::Ne, 2)),
        [b'<', b'=', ..] => Some((CmpOp::Le, 2)),
        [b'>', b'=', ..] => Some((CmpOp::Ge, 2)),
        [b'<', ..] => Some((CmpOp::Lt, 1)),
        [b'>', ..] => Some((CmpOp::Gt, 1)),
        _ => None,
    }
}

fn run(bytes: &[u8], start: usize, keep: fn(u8) -> bool) -> usize {
    let mut at = start;
    while bytes.get(at).copied().is_some_and(keep) {
        at += 1;
    }
    at
}

/// Reads the string starting at the `"` at `start`; returns it and the index after the closing quote.
fn string(text: &str, start: usize) -> Result<(String, usize), QueryError> {
    let mut value = String::new();
    let mut chars = text[start + 1..].char_indices();
    while let Some((offset, current)) = chars.next() {
        match current {
            '"' => return Ok((value, start + 1 + offset + 1)),
            '\\' => {
                let escaped = chars.next().map(|(_, found)| found);
                match escaped {
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some('n') => value.push('\n'),
                    Some('t') => value.push('\t'),
                    other => {
                        let from = start + 1 + offset;
                        let width = 1 + other.map_or(0, char::len_utf8);
                        return Err(QueryError::new(
                            "unknown escape in string",
                            from..from + width,
                        ));
                    }
                }
            }
            other => value.push(other),
        }
    }
    Err(QueryError::new("unterminated string", start..text.len()))
}
