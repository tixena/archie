//! YAML frontmatter: block and flow collections, block scalars, tags and
//! plain scalars into a mapping.

use core::ops::Range;

use indexmap::IndexMap;

use crate::parser::ast::{Mapping, Pair, Scalar, ScalarValue, Sequence, Value};
use crate::parser::cursor::Cursor;
use crate::parser::error::ParseError;
use crate::parser::frontmatter::{finish, insert};
use crate::parser::scalar;

/// The core-schema tags archie reads, plus `!!timestamp`.
const TAGS: [&str; 8] = [
    "bool",
    "float",
    "int",
    "map",
    "null",
    "seq",
    "str",
    "timestamp",
];

/// What a block scalar keeps of the line break after its text and of the
/// empty lines after that.
#[derive(Clone, Copy)]
enum Chomping {
    /// The line break only.
    Clip,
    /// The line break and every empty line.
    Keep,
    /// Neither.
    Strip,
}

/// Where a value is written: under a key or a list entry at the given
/// column, or inside a flow collection.
#[derive(Clone, Copy)]
enum Context {
    Block(usize),
    Flow,
}

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
    if !cursor.eat(b':') {
        return cursor.error("expected `:` after the key");
    }
    let after_colon = cursor.pos();
    cursor.skip_spaces();
    let value = if at_line_end(cursor) {
        end_of_line(cursor)?;
        nested(cursor, indent, after_colon)?
    } else {
        let inline = inline_value(cursor, Context::Block(indent))?;
        end_of_line(cursor)?;
        inline
    };
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
        Some(b'"') => double_quoted(cursor),
        Some(b'\'') => single_quoted(cursor),
        Some(b'!') => tagged_key(cursor, key),
        Some(b'&' | b'*') => anchor(cursor),
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
            let value = inline_value(cursor, Context::Block(indent))?;
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

/// A value that starts at the cursor. It ends on its own line, except a flow
/// collection, which may run over several, and a block scalar, which takes
/// the lines under it; either way the cursor stops after its last text.
fn inline_value(cursor: &mut Cursor<'_>, context: Context) -> Result<Value, ParseError> {
    let start = cursor.pos();
    match cursor.peek() {
        Some(b'[') => flow_sequence(cursor),
        Some(b'{') => flow_mapping(cursor),
        Some(b'"') => {
            let text = double_quoted(cursor)?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'\'') => {
            let text = single_quoted(cursor)?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'|' | b'>') => match context {
            Context::Block(indent) => block_scalar(cursor, indent),
            Context::Flow => plain(cursor, context),
        },
        Some(b'!') => tagged(cursor, context),
        Some(b'&' | b'*') => anchor(cursor),
        Some(_) | None => plain(cursor, context),
    }
}

fn flow_sequence(cursor: &mut Cursor<'_>) -> Result<Value, ParseError> {
    let start = cursor.pos();
    cursor.bump();
    let mut items = Vec::new();
    loop {
        skip_flow_space(cursor);
        if cursor.eat(b']') {
            break;
        }
        items.push(inline_value(cursor, Context::Flow)?);
        skip_flow_space(cursor);
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

fn flow_mapping(cursor: &mut Cursor<'_>) -> Result<Value, ParseError> {
    let start = cursor.pos();
    cursor.bump();
    let mut pairs = IndexMap::new();
    loop {
        skip_flow_space(cursor);
        if cursor.eat(b'}') {
            break;
        }
        flow_entry(cursor, &mut pairs)?;
        skip_flow_space(cursor);
        if cursor.eat(b'}') {
            break;
        }
        if !cursor.eat(b',') {
            return cursor.error("expected `,` or `}` in the mapping");
        }
    }
    Ok(Value::Mapping(Mapping {
        pairs,
        span: start..cursor.pos(),
    }))
}

/// One pair of a flow mapping. A key with no `:`, or with nothing after it,
/// holds null.
fn flow_entry(
    cursor: &mut Cursor<'_>,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), ParseError> {
    let start = cursor.pos();
    let name = flow_key(cursor)?;
    let key_end = cursor.pos();
    cursor.skip_spaces();
    let value = if cursor.eat(b':') {
        let after_colon = cursor.pos();
        skip_flow_space(cursor);
        if matches!(cursor.peek(), Some(b',' | b'}')) {
            null(after_colon)
        } else {
            inline_value(cursor, Context::Flow)?
        }
    } else {
        null(key_end)
    };
    let pair = Pair {
        repeats: Vec::new(),
        span: start..value.span().end,
        value,
    };
    insert(pairs, name, pair);
    Ok(())
}

/// A key inside a flow mapping. A plain key ends at a `:` followed by a
/// space or a flow indicator, at a flow indicator, or at a comment.
fn flow_key(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    match cursor.peek() {
        Some(b'"') => double_quoted(cursor),
        Some(b'\'') => single_quoted(cursor),
        Some(b'!') => tagged_key(cursor, flow_key),
        Some(b'&' | b'*') => anchor(cursor),
        Some(_) | None => {
            let line = cursor.rest_of_line();
            let bytes = line.as_bytes();
            let length = (0..bytes.len())
                .find(|&at| match bytes[at] {
                    b',' | b'[' | b']' | b'{' | b'}' => true,
                    b':' => matches!(
                        bytes.get(at + 1),
                        None | Some(b' ' | b'\t' | b',' | b'[' | b']' | b'{' | b'}')
                    ),
                    b' ' | b'\t' => bytes.get(at + 1) == Some(&b'#'),
                    _ => false,
                })
                .unwrap_or(bytes.len());
            let text = line[..length].trim_end();
            if text.is_empty() {
                return cursor.error("expected a key");
            }
            cursor.set_pos(cursor.pos() + text.len());
            Ok(text.to_owned())
        }
    }
}

/// Skips the spaces, line breaks and comments between the entries of a
/// flow collection.
fn skip_flow_space(cursor: &mut Cursor<'_>) {
    loop {
        match cursor.peek() {
            Some(b' ' | b'\t' | b'\r' | b'\n') => cursor.bump(),
            Some(b'#') => {
                let _: &str = cursor.take_until(b"\n");
            }
            Some(_) | None => break,
        }
    }
}

/// An unquoted scalar: the rest of the line up to a comment, or inside a
/// flow collection up to the next `,`, `[`, `]`, `{` or `}`.
fn plain(cursor: &mut Cursor<'_>, context: Context) -> Result<Value, ParseError> {
    let start = cursor.pos();
    let line = cursor.rest_of_line();
    let uncommented = line.find(" #").map_or(line, |at| &line[..at]);
    let bounded = match context {
        Context::Block(_) => uncommented,
        Context::Flow => uncommented
            .find([',', '[', ']', '{', '}'])
            .map_or(uncommented, |at| &uncommented[..at]),
    };
    let text = bounded.trim_end();
    if text.contains(": ") || text.ends_with(':') {
        return cursor.error("a plain value cannot hold `: `; quote it");
    }
    let end = start + text.len();
    cursor.set_pos(end);
    Ok(Value::Scalar(Scalar {
        span: start..end,
        value: scalar::yaml(text),
    }))
}

/// A literal (`|`) or folded (`>`) block scalar whose key or list entry sits
/// at column `indent`: its header, then the lines indented more than that.
fn block_scalar(cursor: &mut Cursor<'_>, indent: usize) -> Result<Value, ParseError> {
    let start = cursor.pos();
    let folded = cursor.peek() == Some(b'>');
    cursor.bump();
    let (chomping, explicit) = block_header(cursor);
    let mut end = cursor.pos();
    end_of_line(cursor)?;
    let width = match explicit {
        Some(digits) => indent + digits,
        None => detected_width(cursor, indent)?,
    };
    let content_start = cursor.pos();
    let mut offset = 0;
    let mut lines = Vec::new();
    let mut broken = false;
    for piece in cursor.rest().split_inclusive('\n') {
        let line = line_text(piece);
        let spaces = leading_spaces(line);
        let has_break = piece.ends_with('\n');
        if is_blank(line) && spaces <= width {
            if has_break {
                lines.push(None);
            }
        } else if spaces >= width {
            lines.push(Some(&line[width..]));
            end = content_start + offset + line.len();
            broken = has_break;
        } else {
            break;
        }
        offset += piece.len();
    }
    let text_lines = lines
        .iter()
        .rposition(Option::is_some)
        .map_or(0, |at| at + 1);
    let (body, trailing) = lines.split_at(text_lines);
    let mut text = if folded { fold(body) } else { literal(body) };
    let final_break = usize::from(broken);
    let breaks = match chomping {
        Chomping::Clip => final_break,
        Chomping::Keep => final_break + trailing.len(),
        Chomping::Strip => 0,
    };
    text.push_str(&"\n".repeat(breaks));
    cursor.set_pos(end);
    Ok(string(start..end, text))
}

/// The chomping indicator and the indentation digit after `|` or `>`, in
/// either order.
fn block_header(cursor: &mut Cursor<'_>) -> (Chomping, Option<usize>) {
    let mut chomping = Chomping::Clip;
    let mut explicit = None;
    for _ in 0_u8..2 {
        match cursor.peek() {
            Some(b'-') if matches!(chomping, Chomping::Clip) => chomping = Chomping::Strip,
            Some(b'+') if matches!(chomping, Chomping::Clip) => chomping = Chomping::Keep,
            Some(digit @ b'1'..=b'9') if explicit.is_none() => {
                explicit = Some(usize::from(digit - b'0'));
            }
            Some(_) | None => break,
        }
        cursor.bump();
    }
    (chomping, explicit)
}

/// The column of a block scalar's text, from its first line of text. With
/// no text, every blank line is empty.
fn detected_width(cursor: &Cursor<'_>, indent: usize) -> Result<usize, ParseError> {
    let mut deepest_blank = 0;
    let mut offset = 0;
    for piece in cursor.rest().split_inclusive('\n') {
        let line = line_text(piece);
        let spaces = leading_spaces(line);
        if is_blank(line) {
            deepest_blank = deepest_blank.max(spaces);
        } else if spaces <= indent {
            break;
        } else if deepest_blank > spaces {
            let at = cursor.pos() + offset;
            return Err(ParseError::new(
                "a blank line before the text of a block scalar is indented more than the text",
                at..at,
            ));
        } else {
            return Ok(spaces);
        }
        offset += piece.len();
    }
    Ok(deepest_blank.max(indent + 1))
}

/// Literal lines keep their line breaks.
fn literal(lines: &[Option<&str>]) -> String {
    let texts: Vec<&str> = lines.iter().map(|line| line.unwrap_or_default()).collect();
    texts.join("\n")
}

/// Folded lines: the line break between two lines of text is a space, an
/// empty line is a line break, and a line indented more than the text keeps
/// the line breaks around it.
fn fold(lines: &[Option<&str>]) -> String {
    let mut text = String::new();
    let mut previous: Option<&str> = None;
    let mut empty = 0;
    for line in lines {
        let Some(current) = *line else {
            empty += 1;
            continue;
        };
        let breaks = match previous {
            Some(before) if !is_spaced(before) && !is_spaced(current) => {
                if empty == 0 {
                    text.push(' ');
                }
                empty
            }
            Some(_) => empty + 1,
            None => empty,
        };
        text.push_str(&"\n".repeat(breaks));
        text.push_str(current);
        previous = Some(current);
        empty = 0;
    }
    text
}

/// A line of `piece` without its line ending.
fn line_text(piece: &str) -> &str {
    let line = piece.strip_suffix('\n').unwrap_or(piece);
    line.strip_suffix('\r').unwrap_or(line)
}

fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_blank(line: &str) -> bool {
    line.trim_start_matches([' ', '\t']).is_empty()
}

/// Whether a line of a folded scalar is indented more than its text.
fn is_spaced(line: &str) -> bool {
    line.starts_with([' ', '\t'])
}

/// A value written after a tag, read again under it. In block context a tag
/// may end its line, and the value is the block under it.
fn tagged(cursor: &mut Cursor<'_>, context: Context) -> Result<Value, ParseError> {
    let name = tag(cursor)?;
    let tag_end = cursor.pos();
    let value = match context {
        Context::Block(indent) => {
            cursor.skip_spaces();
            if at_line_end(cursor) {
                end_of_line(cursor)?;
                let block = nested(cursor, indent, tag_end)?;
                cursor.set_pos(block.span().end);
                block
            } else {
                inline_value(cursor, context)?
            }
        }
        Context::Flow => {
            skip_flow_space(cursor);
            inline_value(cursor, context)?
        }
    };
    retag(cursor, name, value)
}

/// A key written after a tag, read by `read` and checked against the tag.
fn tagged_key(
    cursor: &mut Cursor<'_>,
    read: fn(&mut Cursor<'_>) -> Result<String, ParseError>,
) -> Result<String, ParseError> {
    let name = tag(cursor)?;
    cursor.skip_spaces();
    let start = cursor.pos();
    let text = read(cursor)?;
    let _: Value = retag(cursor, name, string(start..cursor.pos(), text.clone()))?;
    Ok(text)
}

/// Reads the tag at the cursor and returns the type it names: `!!int` and
/// `!<tag:yaml.org,2002:int>` both name `int`.
fn tag(cursor: &mut Cursor<'_>) -> Result<&'static str, ParseError> {
    let start = cursor.pos();
    if cursor.rest().starts_with("!<") {
        let _: &str = cursor.take_until(b" \t\r\n>");
        let _: bool = cursor.eat(b'>');
    } else {
        let _: &str = cursor.take_until(b" \t\r\n,[]{}");
    }
    let written = cursor.slice(start..cursor.pos());
    let named = written.strip_prefix("!!").or_else(|| {
        written
            .strip_prefix("!<tag:yaml.org,2002:")?
            .strip_suffix('>')
    });
    named
        .and_then(|suffix| TAGS.into_iter().find(|known| *known == suffix))
        .ok_or_else(|| {
            let message = format!("the tag `{written}` is not one archie reads");
            ParseError::new(&message, start..cursor.pos())
        })
}

/// `value` read again under the tag `!!name`: a scalar's text gets the
/// tag's type, and a mapping or a list must be what the tag names.
fn retag(cursor: &Cursor<'_>, name: &str, value: Value) -> Result<Value, ParseError> {
    let scalar = match value {
        Value::Mapping(_) if name == "map" => return Ok(value),
        Value::Sequence(_) if name == "seq" => return Ok(value),
        Value::Mapping(mapping) => return Err(misfit("a mapping", name, mapping.span)),
        Value::Sequence(sequence) => return Err(misfit("a list", name, sequence.span)),
        Value::Scalar(scalar) => scalar,
    };
    let text = match scalar.value {
        ScalarValue::String(text) => text,
        ScalarValue::Boolean(_)
        | ScalarValue::Date(_)
        | ScalarValue::DateTime(_)
        | ScalarValue::Float(_)
        | ScalarValue::Integer(_)
        | ScalarValue::Null
        | ScalarValue::Time(_) => cursor.slice(scalar.span.clone()).to_owned(),
    };
    let Some(typed) = typed(name, &text) else {
        return Err(misfit(&format!("`{text}`"), name, scalar.span));
    };
    Ok(Value::Scalar(Scalar {
        span: scalar.span,
        value: typed,
    }))
}

/// The value `text` has under the scalar tag `!!name`, read by the rules of
/// an untagged value of that type, or `None` when it does not fit.
fn typed(name: &str, text: &str) -> Option<ScalarValue> {
    match (name, scalar::yaml(text)) {
        ("str", _) => Some(ScalarValue::String(text.to_owned())),
        ("float", ScalarValue::Integer(_)) => text.parse().ok().map(ScalarValue::Float),
        ("bool", fitting @ ScalarValue::Boolean(_))
        | ("float", fitting @ ScalarValue::Float(_))
        | ("int", fitting @ ScalarValue::Integer(_))
        | ("null", fitting @ ScalarValue::Null)
        | ("timestamp", fitting @ (ScalarValue::Date(_) | ScalarValue::DateTime(_))) => {
            Some(fitting)
        }
        _ => None,
    }
}

fn misfit(written: &str, name: &str, span: Range<usize>) -> ParseError {
    let message = format!("{written} is not a value of the tag `!!{name}`");
    ParseError::new(&message, span)
}

/// Anchors and aliases are refused: reading one means copying a value.
fn anchor<T>(cursor: &Cursor<'_>) -> Result<T, ParseError> {
    let at = cursor.pos();
    Err(ParseError::new(
        "anchors and aliases are not read",
        at..at + 1,
    ))
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
