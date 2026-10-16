//! YAML frontmatter: block mappings, sequences and scalars into a mapping.

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
    let Some(indent) = next_line(&mut cursor) else {
        return Ok(finish(IndexMap::new(), empty_at));
    };
    let mapping = block_mapping(&mut cursor, indent)?;
    if next_line(&mut cursor).is_some() {
        return cursor.error("this line is indented less than the first key");
    }
    Ok(mapping)
}

/// Skips blank lines and comment lines. Leaves the cursor at the start of
/// the next line with content and returns that line's indentation.
fn next_line(cursor: &mut Cursor<'_>) -> Option<usize> {
    loop {
        if cursor.at_end() {
            return None;
        }
        let line = cursor.rest_of_line();
        let content = line.trim_start_matches(' ');
        if !content.trim_end().is_empty() && !content.starts_with('#') {
            return Some(line.len() - content.len());
        }
        let _: &str = cursor.take_until(b"\n");
        cursor.bump();
    }
}

/// The pairs written at exactly `indent`, starting at the cursor's line.
fn block_mapping(cursor: &mut Cursor<'_>, indent: usize) -> Result<Mapping, ParseError> {
    let empty_at = cursor.pos();
    let mut pairs = IndexMap::new();
    mapping_lines(cursor, indent, &mut pairs)?;
    Ok(finish(pairs, empty_at))
}

/// The pairs on the lines from the cursor on that are written at exactly
/// `indent`.
fn mapping_lines(
    cursor: &mut Cursor<'_>,
    indent: usize,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), ParseError> {
    while let Some(found) = next_line(cursor) {
        if found < indent {
            break;
        }
        if found > indent {
            return cursor.error("this line is indented more than the keys above it");
        }
        cursor.set_pos(cursor.pos() + indent);
        if is_dash(cursor, 0) {
            return cursor.error("expected a key, found a list entry");
        }
        entry(cursor, indent, pairs)?;
    }
    Ok(())
}

/// A mapping whose first key is written on a list entry's line, at
/// `column`; its other keys follow on lines indented to that column.
fn entry_mapping(cursor: &mut Cursor<'_>, column: usize) -> Result<Mapping, ParseError> {
    let empty_at = cursor.pos();
    let mut pairs = IndexMap::new();
    entry(cursor, column, &mut pairs)?;
    mapping_lines(cursor, column, &mut pairs)?;
    Ok(finish(pairs, empty_at))
}

fn entry(
    cursor: &mut Cursor<'_>,
    indent: usize,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), ParseError> {
    let start = cursor.pos();
    let name = key(cursor)?;
    let key_end = cursor.pos();
    if !cursor.eat(b':') {
        return cursor.error("expected `:` after the key");
    }
    let after_colon = cursor.pos();
    cursor.skip_spaces();
    let value = if at_line_end(cursor) {
        end_of_line(cursor)?;
        nested(cursor, indent, after_colon)?
    } else {
        let inline = inline_value(cursor, false)?;
        end_of_line(cursor)?;
        inline
    };
    let span = start..value.span().end;
    insert(pairs, name, start..key_end, Pair { span, value })
}

fn key(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    match cursor.peek() {
        Some(b'"') => double_quoted(cursor),
        Some(b'\'') => single_quoted(cursor),
        Some(_) | None => {
            let line = cursor.rest_of_line();
            let Some(length) = plain_key_length(line) else {
                return cursor.error("expected `key: value`");
            };
            cursor.set_pos(cursor.pos() + length);
            Ok(line[..length].trim_end().to_owned())
        }
    }
}

/// A plain key ends at the first `:` followed by a space or the end of the
/// line.
fn plain_key_length(line: &str) -> Option<usize> {
    line.match_indices(':')
        .map(|(at, _)| at)
        .find(|&at| matches!(line.as_bytes().get(at + 1), None | Some(b' ' | b'\t')))
        .filter(|&at| at > 0)
}

/// Whether the cursor is on a plain `key:` that opens a mapping.
fn starts_mapping(cursor: &Cursor<'_>) -> bool {
    !matches!(
        cursor.peek(),
        Some(b'[' | b'{' | b'"' | b'\'' | b'|' | b'>' | b'&' | b'*' | b'!')
    ) && plain_key_length(cursor.rest_of_line()).is_some()
}

/// The value of a key with nothing after its colon: the block that follows
/// it, or null.
fn nested(cursor: &mut Cursor<'_>, indent: usize, null_at: usize) -> Result<Value, ParseError> {
    let Some(found) = next_line(cursor) else {
        return Ok(null(null_at));
    };
    let dash = is_dash(cursor, found);
    if found > indent && dash {
        block_sequence(cursor, found)
    } else if found > indent {
        Ok(Value::Mapping(block_mapping(cursor, found)?))
    } else if found == indent && dash {
        block_sequence(cursor, found)
    } else {
        Ok(null(null_at))
    }
}

fn block_sequence(cursor: &mut Cursor<'_>, indent: usize) -> Result<Value, ParseError> {
    let start = cursor.pos() + indent;
    let mut end = start;
    let mut items = Vec::new();
    while let Some(found) = next_line(cursor) {
        if found > indent {
            return cursor.error("this line is indented more than the list above it");
        }
        if found < indent || !is_dash(cursor, found) {
            break;
        }
        let line_start = cursor.pos();
        cursor.set_pos(line_start + indent + 1);
        let after_dash = cursor.pos();
        cursor.skip_spaces();
        let item = if at_line_end(cursor) {
            end_of_line(cursor)?;
            null(after_dash)
        } else if starts_mapping(cursor) {
            Value::Mapping(entry_mapping(cursor, cursor.pos() - line_start)?)
        } else {
            let value = inline_value(cursor, false)?;
            end_of_line(cursor)?;
            value
        };
        end = item.span().end;
        items.push(item);
    }
    Ok(Value::Sequence(Sequence {
        items,
        span: start..end,
    }))
}

fn inline_value(cursor: &mut Cursor<'_>, flow: bool) -> Result<Value, ParseError> {
    let start = cursor.pos();
    match cursor.peek() {
        Some(b'[') => flow_sequence(cursor),
        Some(b'"') => {
            let text = double_quoted(cursor)?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'\'') => {
            let text = single_quoted(cursor)?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'{') => cursor.error("flow mappings are not read yet"),
        Some(b'|' | b'>') => cursor.error("block scalars are not read yet"),
        Some(b'&' | b'*' | b'!') => cursor.error("anchors, aliases and tags are not read yet"),
        Some(_) | None => plain(cursor, flow),
    }
}

fn flow_sequence(cursor: &mut Cursor<'_>) -> Result<Value, ParseError> {
    let start = cursor.pos();
    cursor.bump();
    let mut items = Vec::new();
    loop {
        cursor.skip_spaces();
        if cursor.eat(b']') {
            break;
        }
        items.push(inline_value(cursor, true)?);
        cursor.skip_spaces();
        if cursor.eat(b']') {
            break;
        }
        if !cursor.eat(b',') {
            return cursor.error("expected `,` or `]` in the list");
        }
    }
    Ok(Value::Sequence(Sequence {
        items,
        span: start..cursor.pos(),
    }))
}

/// An unquoted scalar: the rest of the line up to a comment, or inside a
/// flow sequence up to the next `,` or `]`.
fn plain(cursor: &mut Cursor<'_>, flow: bool) -> Result<Value, ParseError> {
    let start = cursor.pos();
    let line = cursor.rest_of_line();
    let uncommented = line.find(" #").map_or(line, |at| &line[..at]);
    let bounded = if flow {
        uncommented
            .find([',', ']'])
            .map_or(uncommented, |at| &uncommented[..at])
    } else {
        uncommented
    };
    let text = bounded.trim_end();
    if !flow && (text.contains(": ") || text.ends_with(':')) {
        return cursor.error("a mapping written inside a value is not read yet");
    }
    let end = start + text.len();
    cursor.set_pos(end);
    Ok(Value::Scalar(Scalar {
        span: start..end,
        value: scalar::yaml(text),
    }))
}

fn double_quoted(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    cursor.bump();
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
        b'0' => Some('\0'),
        b'a' => Some('\u{7}'),
        b'b' => Some('\u{8}'),
        b't' => Some('\t'),
        b'n' => Some('\n'),
        b'v' => Some('\u{b}'),
        b'f' => Some('\u{c}'),
        b'r' => Some('\r'),
        b'e' => Some('\u{1b}'),
        b' ' => Some(' '),
        b'"' => Some('"'),
        b'/' => Some('/'),
        b'\\' => Some('\\'),
        b'x' => cursor.hex_char(2),
        b'u' => cursor.hex_char(4),
        b'U' => cursor.hex_char(8),
        _ => None,
    };
    character.map_or_else(|| cursor.error("unknown escape in the string"), Ok)
}

fn single_quoted(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    cursor.bump();
    let mut text = String::new();
    loop {
        text.push_str(cursor.take_until(b"'\n"));
        if !cursor.eat(b'\'') {
            return cursor.error("the string is not closed");
        }
        // Inside single quotes, `''` is one quote.
        if !cursor.eat(b'\'') {
            return Ok(text);
        }
        text.push('\'');
    }
}

const fn string(span: Range<usize>, text: String) -> Value {
    Value::Scalar(Scalar {
        span,
        value: ScalarValue::String(text),
    })
}

const fn null(at: usize) -> Value {
    Value::Scalar(Scalar {
        span: at..at,
        value: ScalarValue::Null,
    })
}

/// Whether a list entry starts `offset` bytes after the cursor.
fn is_dash(cursor: &Cursor<'_>, offset: usize) -> bool {
    cursor.peek_at(offset) == Some(b'-')
        && matches!(
            cursor.peek_at(offset + 1),
            None | Some(b' ' | b'\n' | b'\r')
        )
}

fn at_line_end(cursor: &Cursor<'_>) -> bool {
    matches!(cursor.peek(), None | Some(b'\n' | b'\r' | b'#'))
}

fn end_of_line(cursor: &mut Cursor<'_>) -> Result<(), ParseError> {
    cursor.skip_spaces();
    if cursor.peek() == Some(b'#') {
        let _: &str = cursor.take_until(b"\n");
    }
    let _: bool = cursor.eat(b'\r');
    if cursor.at_end() || cursor.eat(b'\n') {
        Ok(())
    } else {
        cursor.error("expected the end of the line after the value")
    }
}
