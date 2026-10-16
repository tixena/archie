//! TOML frontmatter: `key = value` lines into a mapping.

use core::ops::Range;

use indexmap::IndexMap;

use crate::parser::ast::{Mapping, Pair, Scalar, ScalarValue, Sequence, Value};
use crate::parser::cursor::Cursor;
use crate::parser::error::ParseError;
use crate::parser::frontmatter::{finish, insert};
use crate::parser::scalar;

pub fn parse(source: &str, content: Range<usize>) -> Result<Mapping, ParseError> {
    let empty_at = content.start;
    let mut cursor = Cursor::new(source, content);
    let mut pairs = IndexMap::new();
    loop {
        skip_trivia(&mut cursor);
        if cursor.at_end() {
            break;
        }
        if cursor.peek() == Some(b'[') {
            return cursor.error("table headers are not read yet");
        }
        entry(&mut cursor, &mut pairs)?;
        end_of_line(&mut cursor)?;
    }
    Ok(finish(pairs, empty_at))
}

fn entry(cursor: &mut Cursor<'_>, pairs: &mut IndexMap<String, Pair>) -> Result<(), ParseError> {
    let start = cursor.pos();
    let name = key(cursor)?;
    cursor.skip_spaces();
    if cursor.peek() == Some(b'.') {
        return cursor.error("dotted keys are not read yet");
    }
    if !cursor.eat(b'=') {
        return cursor.error("expected `=` after the key");
    }
    cursor.skip_spaces();
    let value = value(cursor)?;
    let pair = Pair {
        repeats: Vec::new(),
        span: start..value.span().end,
        value,
    };
    insert(pairs, name, pair);
    Ok(())
}

fn key(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    match cursor.peek() {
        Some(b'"') => basic_string(cursor),
        Some(b'\'') => literal_string(cursor),
        Some(_) | None => {
            let start = cursor.pos();
            while cursor
                .peek()
                .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
            {
                cursor.bump();
            }
            if cursor.pos() == start {
                return cursor.error("expected a key");
            }
            Ok(cursor.slice(start..cursor.pos()).to_owned())
        }
    }
}

fn value(cursor: &mut Cursor<'_>) -> Result<Value, ParseError> {
    let start = cursor.pos();
    match cursor.peek() {
        Some(b'"') => {
            let text = basic_string(cursor)?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'\'') => {
            let text = literal_string(cursor)?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'[') => array(cursor),
        Some(b'{') => inline_table(cursor),
        Some(_) | None => bare(cursor),
    }
}

const fn string(span: Range<usize>, text: String) -> Value {
    Value::Scalar(Scalar {
        span,
        value: ScalarValue::String(text),
    })
}

fn basic_string(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    cursor.bump();
    if cursor.rest().starts_with("\"\"") {
        return cursor.error("multi-line strings are not read yet");
    }
    let mut text = String::new();
    loop {
        text.push_str(cursor.take_until(b"\"\\\n"));
        if cursor.eat(b'"') {
            return Ok(text);
        }
        if !cursor.eat(b'\\') {
            return cursor.error("the string is not closed");
        }
        text.push(escape(cursor)?);
    }
}

fn escape(cursor: &mut Cursor<'_>) -> Result<char, ParseError> {
    let Some(byte) = cursor.peek() else {
        return cursor.error("the string is not closed");
    };
    cursor.bump();
    let character = match byte {
        b'b' => Some('\u{8}'),
        b't' => Some('\t'),
        b'n' => Some('\n'),
        b'f' => Some('\u{c}'),
        b'r' => Some('\r'),
        b'"' => Some('"'),
        b'\\' => Some('\\'),
        b'u' => cursor.hex_char(4),
        b'U' => cursor.hex_char(8),
        _ => None,
    };
    character.map_or_else(|| cursor.error("unknown escape in the string"), Ok)
}

fn literal_string(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    cursor.bump();
    if cursor.rest().starts_with("''") {
        return cursor.error("multi-line strings are not read yet");
    }
    let text = cursor.take_until(b"'\n").to_owned();
    if !cursor.eat(b'\'') {
        return cursor.error("the string is not closed");
    }
    Ok(text)
}

fn array(cursor: &mut Cursor<'_>) -> Result<Value, ParseError> {
    let start = cursor.pos();
    cursor.bump();
    let mut items = Vec::new();
    loop {
        skip_trivia(cursor);
        if cursor.eat(b']') {
            break;
        }
        items.push(value(cursor)?);
        skip_trivia(cursor);
        if cursor.eat(b']') {
            break;
        }
        if !cursor.eat(b',') {
            return cursor.error("expected `,` or `]` in the array");
        }
    }
    Ok(Value::Sequence(Sequence {
        items,
        span: start..cursor.pos(),
    }))
}

fn inline_table(cursor: &mut Cursor<'_>) -> Result<Value, ParseError> {
    let start = cursor.pos();
    cursor.bump();
    cursor.skip_spaces();
    let mut pairs = IndexMap::new();
    if !cursor.eat(b'}') {
        loop {
            cursor.skip_spaces();
            entry(cursor, &mut pairs)?;
            cursor.skip_spaces();
            if cursor.eat(b'}') {
                break;
            }
            if !cursor.eat(b',') {
                return cursor.error("expected `,` or `}` in the inline table");
            }
        }
    }
    Ok(Value::Mapping(Mapping {
        pairs,
        span: start..cursor.pos(),
    }))
}

/// An unquoted value: a boolean, a number, a date or a time.
fn bare(cursor: &mut Cursor<'_>) -> Result<Value, ParseError> {
    let start = cursor.pos();
    let date_end = token_end(cursor, start);
    if date_end == start {
        return cursor.error("expected a value");
    }
    // A date and its time may be separated by one space instead of `T`.
    let end = if scalar::is_date(cursor.slice(start..date_end)) && time_follows(cursor, date_end) {
        token_end(cursor, date_end + 1)
    } else {
        date_end
    };
    let text = cursor.slice(start..end);
    let Some(value) = scalar::toml(text) else {
        let message = format!("`{text}` is not a value");
        return Err(ParseError::new(&message, start..end));
    };
    cursor.set_pos(end);
    Ok(Value::Scalar(Scalar {
        span: start..end,
        value,
    }))
}

fn token_end(cursor: &Cursor<'_>, from: usize) -> usize {
    let text = cursor.slice(from..cursor.end());
    from + text
        .find([',', ']', '}', '#', ' ', '\t', '\n', '\r'])
        .unwrap_or(text.len())
}

fn time_follows(cursor: &Cursor<'_>, at: usize) -> bool {
    let bytes = cursor.slice(at..cursor.end()).as_bytes();
    matches!(bytes, [b' ', hour_tens, hour_units, b':', ..] if hour_tens.is_ascii_digit() && hour_units.is_ascii_digit())
}

fn skip_trivia(cursor: &mut Cursor<'_>) {
    loop {
        match cursor.peek() {
            Some(b' ' | b'\t' | b'\n' | b'\r') => cursor.bump(),
            Some(b'#') => skip_comment(cursor),
            Some(_) | None => break,
        }
    }
}

fn skip_comment(cursor: &mut Cursor<'_>) {
    let _: &str = cursor.take_until(b"\n");
}

fn end_of_line(cursor: &mut Cursor<'_>) -> Result<(), ParseError> {
    cursor.skip_spaces();
    if cursor.peek() == Some(b'#') {
        skip_comment(cursor);
    }
    match cursor.peek() {
        None | Some(b'\n' | b'\r') => Ok(()),
        Some(_) => cursor.error("expected the end of the line after the value"),
    }
}
