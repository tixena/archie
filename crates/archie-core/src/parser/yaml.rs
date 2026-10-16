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

/// Where a value is written: after a key or in a list entry whose `-` is
/// at the given column, or inside a flow collection whose lines must be
/// indented more than the given column, if any.
#[derive(Clone, Copy)]
enum Context {
    Entry(usize),
    Flow(Option<usize>),
    Key(usize),
}

impl Context {
    /// The column the lines of a value written here must be indented more
    /// than; none at the root of the frontmatter.
    const fn column(self) -> Option<usize> {
        match self {
            Self::Entry(column) | Self::Key(column) => Some(column),
            Self::Flow(column) => column,
        }
    }
}

/// A tag archie reads: the type it names and where it is written.
struct Tag {
    name: &'static str,
    span: Range<usize>,
}

pub fn parse(source: &str, content: Range<usize>) -> Result<Mapping, ParseError> {
    let empty_at = content.start;
    let mut cursor = Cursor::new(source, content);
    let Some(indent) = next_line(&mut cursor) else {
        return Ok(finish(IndexMap::new(), empty_at));
    };
    let root = cursor.pos() + indent;
    match cursor.peek_at(indent) {
        Some(b'{') => {
            cursor.set_pos(root);
            let mapping = flow_mapping(&mut cursor, None)?;
            end_of_line(&mut cursor)?;
            if next_line(&mut cursor).is_some() {
                return cursor.error("expected the end of the frontmatter after the mapping");
            }
            Ok(mapping)
        }
        Some(b'[') => {
            cursor.set_pos(root);
            let list = flow_sequence(&mut cursor, None)?;
            Err(not_a_mapping(list.span()))
        }
        Some(_) | None => {
            let mapping = block_mapping(&mut cursor, indent).map_err(|error| {
                cursor.set_pos(root);
                lone_value(&mut cursor, indent).map_or(error, |span| not_a_mapping(&span))
            })?;
            if next_line(&mut cursor).is_some() {
                return cursor.error("this line is indented less than the first key");
            }
            Ok(mapping)
        }
    }
}

/// The span of the value at the cursor when it fills its line, as a scalar
/// or a list does and a key does not.
fn lone_value(cursor: &mut Cursor<'_>, indent: usize) -> Option<Range<usize>> {
    let value = if is_dash(cursor, 0) {
        block_sequence(cursor, cursor.pos() - indent, indent).ok()?
    } else {
        let value = inline_value(cursor, Context::Key(indent)).ok()?;
        end_of_line(cursor).ok()?;
        value
    };
    Some(value.span().clone())
}

fn not_a_mapping(span: &Range<usize>) -> ParseError {
    ParseError::new("the frontmatter must be a mapping", span.clone())
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
        let text = content.trim_start_matches('\t');
        if !text.trim_end().is_empty() && !text.starts_with('#') {
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
        if cursor.peek() == Some(b'\t') {
            return Err(tab_indent(cursor.pos()));
        }
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

/// One pair of a block mapping whose keys are at column `indent`: a `key:`
/// and its value, a `:` and its value with the empty key, or an explicit key
/// after `?`.
fn entry(
    cursor: &mut Cursor<'_>,
    indent: usize,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), ParseError> {
    if is_explicit_key(cursor) {
        return explicit_entry(cursor, indent, pairs);
    }
    let start = cursor.pos();
    let name = if is_empty_key(cursor, false) {
        String::new()
    } else {
        key(cursor)?
    };
    cursor.skip_spaces();
    if !cursor.eat(b':') {
        return cursor.error("expected `:` after the key");
    }
    if !separated_at(cursor, 0) {
        return cursor.error("expected a space after the `:`");
    }
    let value = indicated_value(cursor, indent)?;
    let pair = Pair {
        repeats: Vec::new(),
        span: start..value.span().end,
        value,
    };
    insert(pairs, name, pair);
    Ok(())
}

/// A pair whose key follows a `?` at column `indent`. Its value follows a
/// `:` at that column on the next line with content; with none, it is null
/// at the end of the key. Both lines are read as a list entry's `-` line is.
fn explicit_entry(
    cursor: &mut Cursor<'_>,
    indent: usize,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), ParseError> {
    let start = cursor.pos();
    let key = compact_value(cursor, start - indent, indent, Context::Key(indent))?;
    let key_end = key.span().end;
    let name = explicit_key(cursor, key)?;
    let value = if next_line(cursor) == Some(indent)
        && cursor.peek_at(indent) == Some(b':')
        && separated_at(cursor, indent + 1)
    {
        let line_start = cursor.pos();
        compact_value(cursor, line_start, indent, Context::Key(indent))?
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

/// The text of an explicit key, which must be a scalar.
fn explicit_key(cursor: &Cursor<'_>, key: Value) -> Result<String, ParseError> {
    let span = match key {
        Value::Scalar(scalar) => return Ok(scalar_text(cursor, scalar.value, &scalar.span)),
        Value::Mapping(mapping) => mapping.span,
        Value::Sequence(sequence) => sequence.span,
    };
    Err(ParseError::new("expected a key", span.start..span.start))
}

/// The value after the `:` of a key at column `indent`, with the cursor
/// right after the `:`: the rest of the line, what the lines under it hold,
/// or null right after the `:`.
fn indicated_value(cursor: &mut Cursor<'_>, indent: usize) -> Result<Value, ParseError> {
    let after = cursor.pos();
    cursor.skip_spaces();
    if at_line_end(cursor) {
        end_of_line(cursor)?;
        return nested(cursor, Context::Key(indent), after);
    }
    let value = inline_value(cursor, Context::Key(indent))?;
    end_of_line(cursor)?;
    Ok(value)
}

fn key(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    match cursor.peek() {
        Some(b'"') => double_quoted(cursor, None),
        Some(b'\'') => single_quoted(cursor, None),
        Some(b'!') => tagged_key(cursor, key),
        Some(b'&' | b'*') => anchor(cursor),
        Some(b'[' | b'{') => cursor.error("expected a key"),
        Some(_) | None => {
            no_indicator(cursor, false, "key")?;
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

/// Whether the cursor is on a pair that opens a mapping: an explicit key, an
/// empty key, or a `key:` after a tag or not, whose key is plain or quoted.
fn starts_mapping(cursor: &mut Cursor<'_>) -> bool {
    if is_explicit_key(cursor) || is_empty_key(cursor, false) {
        return true;
    }
    let start = cursor.pos();
    if cursor.peek() == Some(b'!') && tag(cursor).is_ok() {
        cursor.skip_spaces();
    }
    let is_mapping = match cursor.peek() {
        Some(b'"' | b'\'') => {
            let is_key = key(cursor).is_ok();
            cursor.skip_spaces();
            is_key && cursor.peek() == Some(b':')
        }
        Some(b'[' | b'{' | b'|' | b'>' | b'&' | b'*' | b'!') => false,
        Some(_) | None => plain_key_length(cursor.rest_of_line()).is_some(),
    };
    cursor.set_pos(start);
    is_mapping
}

/// Whether the cursor is on a `?` that starts an explicit key: one followed
/// by a space, a tab or the end of the line.
fn is_explicit_key(cursor: &Cursor<'_>) -> bool {
    cursor.peek() == Some(b'?') && separated_at(cursor, 1)
}

/// The value of a key or a list entry with nothing after it on its line:
/// what the lines indented more than it hold, a list at a key's own
/// column, or null.
fn nested(cursor: &mut Cursor<'_>, context: Context, null_at: usize) -> Result<Value, ParseError> {
    let Some(found) = next_line(cursor) else {
        return Ok(null(null_at));
    };
    let is_list = is_dash(cursor, found);
    let compact = is_list && matches!(context, Context::Key(column) if column == found);
    if !compact && context.column().is_some_and(|column| found <= column) {
        return Ok(null(null_at));
    }
    let line_start = cursor.pos();
    if is_list {
        return block_sequence(cursor, line_start, found);
    }
    cursor.set_pos(line_start + found);
    no_tab_before_block(cursor)?;
    if starts_mapping(cursor) {
        cursor.set_pos(line_start);
        return Ok(Value::Mapping(block_mapping(cursor, found)?));
    }
    let value = inline_value(cursor, context)?;
    end_of_line(cursor)?;
    Ok(value)
}

/// Refuses the tab at the cursor, after the indentation of a line, when a
/// key, an explicit key or a list entry follows it.
fn no_tab_before_block(cursor: &mut Cursor<'_>) -> Result<(), ParseError> {
    let tab = cursor.pos();
    if cursor.peek() != Some(b'\t') {
        return Ok(());
    }
    cursor.skip_spaces();
    let starts_block = is_dash(cursor, 0) || starts_mapping(cursor);
    cursor.set_pos(tab);
    if starts_block {
        return Err(tab_indent(tab));
    }
    Ok(())
}

/// A tab at `at`, before the first character of a line: a tab is never
/// indentation.
fn tab_indent(at: usize) -> ParseError {
    ParseError::new("a tab cannot indent a line; use spaces", at..at + 1)
}

/// A block list whose first `-` is at column `indent` of the line starting
/// at `line_start`, and whose other entries are the lines below with a `-`
/// at that column.
fn block_sequence(
    cursor: &mut Cursor<'_>,
    line_start: usize,
    indent: usize,
) -> Result<Value, ParseError> {
    let start = line_start + indent;
    let mut entry_line = line_start;
    let mut items = Vec::new();
    loop {
        items.push(compact_value(
            cursor,
            entry_line,
            indent,
            Context::Entry(indent),
        )?);
        let Some(found) = next_line(cursor) else {
            break;
        };
        if found > indent {
            return cursor.error("this line is indented more than the list above it");
        }
        if found == indent && cursor.peek_at(found) == Some(b'\t') {
            return Err(tab_indent(cursor.pos() + found));
        }
        if found < indent || !is_dash(cursor, found) {
            break;
        }
        entry_line = cursor.pos();
    }
    let end = items.last().map_or(start, |item| item.span().end);
    Ok(Value::Sequence(Sequence {
        items,
        span: start..end,
    }))
}

/// The value after the `-`, `?` or `:` at column `indent` of the line
/// starting at `line_start`, written in `context`. A `-` after it starts a
/// list at its own column, and a `key:` or a `?` there a mapping, unless a
/// tab comes before them: a tab is never indentation.
fn compact_value(
    cursor: &mut Cursor<'_>,
    line_start: usize,
    indent: usize,
    context: Context,
) -> Result<Value, ParseError> {
    cursor.set_pos(line_start + indent + 1);
    let after_indicator = cursor.pos();
    cursor.skip_spaces();
    let column = cursor.pos() - line_start;
    if at_line_end(cursor) {
        end_of_line(cursor)?;
        return nested(cursor, context, after_indicator);
    }
    let indented = !cursor.slice(after_indicator..cursor.pos()).contains('\t');
    if indented && is_dash(cursor, 0) {
        return block_sequence(cursor, line_start, column);
    }
    if indented && starts_mapping(cursor) {
        return Ok(Value::Mapping(entry_mapping(cursor, column)?));
    }
    let value = inline_value(cursor, context)?;
    end_of_line(cursor)?;
    Ok(value)
}

/// A value that starts at the cursor. It ends on its own line, except a flow
/// collection, which may run over several, and a block scalar, which takes
/// the lines under it; either way the cursor stops after its last text.
fn inline_value(cursor: &mut Cursor<'_>, context: Context) -> Result<Value, ParseError> {
    let start = cursor.pos();
    match cursor.peek() {
        Some(b'[') => flow_sequence(cursor, context.column()),
        Some(b'{') => flow_mapping(cursor, context.column()).map(Value::Mapping),
        Some(b'"') => {
            let text = double_quoted(cursor, Some(context))?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'\'') => {
            let text = single_quoted(cursor, Some(context))?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'|' | b'>') => match context {
            Context::Entry(indent) | Context::Key(indent) => block_scalar(cursor, indent),
            Context::Flow(_) => plain(cursor, context),
        },
        Some(b'!') => tagged(cursor, context),
        Some(b'&' | b'*') => anchor(cursor),
        Some(_) | None => plain(cursor, context),
    }
}

/// A flow list whose lines are indented more than `column`, the column of
/// its key or `-`; none at the root.
fn flow_sequence(cursor: &mut Cursor<'_>, column: Option<usize>) -> Result<Value, ParseError> {
    let start = cursor.pos();
    cursor.bump();
    let mut items = Vec::new();
    loop {
        skip_flow_space(cursor, column)?;
        if cursor.eat(b']') {
            break;
        }
        no_empty_entry(cursor)?;
        items.push(flow_item(cursor, column)?);
        skip_flow_space(cursor, column)?;
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

/// One entry of a flow list: a value, or a pair, which makes a mapping of
/// one pair.
fn flow_item(cursor: &mut Cursor<'_>, column: Option<usize>) -> Result<Value, ParseError> {
    if !is_flow_pair(cursor) {
        return inline_value(cursor, Context::Flow(column));
    }
    let start = cursor.pos();
    let mut pairs = IndexMap::new();
    flow_entry(cursor, column, &mut pairs)?;
    Ok(Value::Mapping(finish(pairs, start)))
}

/// Whether the flow entry at the cursor is a pair: an explicit key, an empty
/// key or a key followed by `:`.
fn is_flow_pair(cursor: &mut Cursor<'_>) -> bool {
    if is_flow_explicit_key(cursor) || is_empty_key(cursor, true) {
        return true;
    }
    let start = cursor.pos();
    let is_key = flow_key(cursor).is_ok();
    cursor.skip_spaces();
    let is_pair = is_key && cursor.peek() == Some(b':');
    cursor.set_pos(start);
    is_pair
}

/// Refuses a `,` where an entry of a flow collection should start.
fn no_empty_entry(cursor: &Cursor<'_>) -> Result<(), ParseError> {
    if cursor.peek() == Some(b',') {
        let at = cursor.pos();
        return Err(ParseError::new("expected a value before `,`", at..at + 1));
    }
    Ok(())
}

/// A flow mapping whose lines are indented more than `column`, the column
/// of its key or `-`; none at the root.
fn flow_mapping(cursor: &mut Cursor<'_>, column: Option<usize>) -> Result<Mapping, ParseError> {
    let start = cursor.pos();
    cursor.bump();
    let mut pairs = IndexMap::new();
    loop {
        skip_flow_space(cursor, column)?;
        if cursor.eat(b'}') {
            break;
        }
        no_empty_entry(cursor)?;
        flow_entry(cursor, column, &mut pairs)?;
        skip_flow_space(cursor, column)?;
        if cursor.eat(b'}') {
            break;
        }
        if !cursor.eat(b',') {
            return cursor.error("expected `,` or `}` in the mapping");
        }
    }
    Ok(Mapping {
        pairs,
        span: start..cursor.pos(),
    })
}

/// One pair of a flow mapping, or of a flow list. Line breaks and comments
/// may come before its `:`; in a flow list, `is_flow_pair` allows that only
/// after an explicit key. A key with no `:`, or with nothing after it, holds
/// null.
fn flow_entry(
    cursor: &mut Cursor<'_>,
    column: Option<usize>,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), ParseError> {
    let start = cursor.pos();
    let name = flow_pair_key(cursor, column)?;
    let key_end = cursor.pos();
    skip_flow_space(cursor, column)?;
    let value = if cursor.eat(b':') {
        let after_colon = cursor.pos();
        skip_flow_space(cursor, column)?;
        if matches!(cursor.peek(), Some(b',' | b'}' | b']')) {
            null(after_colon)
        } else {
            inline_value(cursor, Context::Flow(column))?
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

/// The key of a flow pair: the key after a `?`, or the empty key when none
/// follows it or when the pair starts with its `:`.
fn flow_pair_key(cursor: &mut Cursor<'_>, column: Option<usize>) -> Result<String, ParseError> {
    let explicit = is_flow_explicit_key(cursor);
    if explicit {
        cursor.bump();
        skip_flow_space(cursor, column)?;
    }
    if is_empty_key(cursor, true)
        || (explicit && matches!(cursor.peek(), None | Some(b',' | b']' | b'}')))
    {
        return Ok(String::new());
    }
    flow_key(cursor)
}

/// Whether the cursor is on a `?` that starts an explicit key inside a flow
/// collection: one followed by a space, a tab, the end of the line, `,`,
/// `]` or `}`.
fn is_flow_explicit_key(cursor: &Cursor<'_>) -> bool {
    cursor.peek() == Some(b'?')
        && (separated_at(cursor, 1) || matches!(cursor.peek_at(1), Some(b',' | b']' | b'}')))
}

/// Whether a pair starts at the cursor with its `:`, with the empty key,
/// inside a flow collection or not.
fn is_empty_key(cursor: &Cursor<'_>, in_flow: bool) -> bool {
    cursor.peek() == Some(b':') && is_indicator(cursor, in_flow)
}

/// A key inside a flow mapping. A plain key ends at a `:` followed by a
/// space or a flow indicator, at a flow indicator, or at a comment.
fn flow_key(cursor: &mut Cursor<'_>) -> Result<String, ParseError> {
    match cursor.peek() {
        Some(b'"') => double_quoted(cursor, None),
        Some(b'\'') => single_quoted(cursor, None),
        Some(b'!') => tagged_key(cursor, flow_key),
        Some(b'&' | b'*') => anchor(cursor),
        Some(_) | None => {
            no_indicator(cursor, true, "key")?;
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
/// flow collection. The line the next entry starts on must be indented
/// more than `column`.
fn skip_flow_space(cursor: &mut Cursor<'_>, column: Option<usize>) -> Result<(), ParseError> {
    let mut line_start = None;
    loop {
        match cursor.peek() {
            Some(b'\n') => {
                cursor.bump();
                line_start = Some(cursor.pos());
            }
            Some(b' ' | b'\t' | b'\r') => cursor.bump(),
            Some(b'#') => {
                let _: &str = cursor.take_until(b"\n");
            }
            Some(_) => break,
            None => return Ok(()),
        }
    }
    if let (Some(start), Some(parent)) = (line_start, column)
        && leading_spaces(cursor.slice(start..cursor.pos())) <= parent
    {
        return cursor.error("a line of a flow collection must be indented more than its key");
    }
    Ok(())
}

/// An unquoted scalar: the rest of the line up to a comment, or inside a
/// flow collection up to the next `,`, `[`, `]`, `{` or `}`. A line that
/// runs to its end goes on over the lines below it that are indented more
/// than its key or `-`.
fn plain(cursor: &mut Cursor<'_>, context: Context) -> Result<Value, ParseError> {
    no_indicator(cursor, matches!(context, Context::Flow(_)), "value")?;
    let start = cursor.pos();
    let line = cursor.rest_of_line();
    let bounded = plain_part(line, context);
    let text = bounded.trim_end();
    if holds_pair(text) {
        return cursor.error("a plain value cannot hold `: `; quote it");
    }
    let first_end = start + text.len();
    let end = if bounded.len() == line.len() {
        plain_end(cursor, first_end, context)
    } else {
        first_end
    };
    cursor.set_pos(end);
    Ok(Value::Scalar(Scalar {
        span: start..end,
        value: scalar::yaml(&folded(cursor.slice(start..end))),
    }))
}

/// Refuses a plain scalar, the `noun` named in the error, that starts with
/// `@`, a backtick, `%`, `|`, `>`, `,`, `]` or `}`, or with a `?`, `:` or `-`
/// that is an indicator. Inside a flow collection, `,`, `]` and `}` end the
/// entry instead.
fn no_indicator(cursor: &Cursor<'_>, in_flow: bool, noun: &str) -> Result<(), ParseError> {
    let Some(first) = cursor.peek() else {
        return Ok(());
    };
    let refused = match first {
        b',' | b']' | b'}' => !in_flow,
        b'@' | b'`' | b'%' | b'|' | b'>' => true,
        b'?' | b':' | b'-' => is_indicator(cursor, in_flow),
        _ => false,
    };
    if !refused {
        return Ok(());
    }
    let spaced =
        matches!(first, b'?' | b':' | b'-') && matches!(cursor.peek_at(1), Some(b' ' | b'\t'));
    let and_space = if spaced { " and a space" } else { "" };
    let message = format!(
        "a plain {noun} cannot start with `{}`{and_space}; quote it",
        char::from(first)
    );
    let at = cursor.pos();
    Err(ParseError::new(&message, at..at + 1))
}

/// The start of `line` that a plain scalar may hold: up to a comment, and
/// inside a flow collection up to the next `,`, `[`, `]`, `{` or `}`.
fn plain_part(line: &str, context: Context) -> &str {
    let uncommented = uncommented(line);
    match context {
        Context::Entry(_) | Context::Key(_) => uncommented,
        Context::Flow(_) => uncommented
            .find([',', '[', ']', '{', '}'])
            .map_or(uncommented, |at| &uncommented[..at]),
    }
}

/// Where a plain scalar whose first line ends at `first_end` ends: after the
/// last line below that goes on with it. Such a line comes after nothing but
/// blank lines, is indented more than the column of `within`, holds no
/// `key:` and starts with no comment or flow indicator; a comment or a flow
/// indicator after its text makes it the last.
fn plain_end(cursor: &Cursor<'_>, first_end: usize, within: Context) -> usize {
    let rest = cursor.slice(first_end..cursor.end());
    let Some(break_at) = rest.find('\n') else {
        return first_end;
    };
    let mut end = first_end;
    let mut line_start = first_end + break_at + 1;
    for piece in cursor.slice(line_start..cursor.end()).split_inclusive('\n') {
        let line = line_text(piece);
        if !is_blank(line) {
            let content = line.trim_start_matches([' ', '\t']);
            let bounded = plain_part(content, within);
            let text = bounded.trim_end();
            let shallow = within
                .column()
                .is_some_and(|column| leading_spaces(line) <= column);
            if shallow || content.starts_with('#') || text.is_empty() || holds_pair(text) {
                break;
            }
            end = line_start + (line.len() - content.len()) + text.len();
            if bounded.len() < content.len() {
                break;
            }
        }
        line_start += piece.len();
    }
    end
}

/// A line without the comment at its end, which starts at a `#` after a
/// space or a tab.
fn uncommented(line: &str) -> &str {
    line.match_indices('#')
        .map(|(at, _)| &line[..at])
        .find(|before| before.ends_with([' ', '\t']))
        .map_or(line, |before| &before[..before.len() - 1])
}

/// Whether a plain scalar's text holds `: ` or ends with `:`, as a key does.
fn holds_pair(text: &str) -> bool {
    text.contains(": ") || text.ends_with(':')
}

/// The text of a plain scalar from the source it covers: each line without
/// the spaces around it, joined by a space, or by one line break for each
/// blank line between them.
fn folded(written: &str) -> String {
    let mut text = String::new();
    let mut blank = 0;
    for (number, line) in written.split('\n').enumerate() {
        let trimmed = line.trim_matches([' ', '\t', '\r']);
        if trimmed.is_empty() {
            blank += 1;
            continue;
        }
        if number > 0 {
            push_break(&mut text, blank);
        }
        text.push_str(trimmed);
        blank = 0;
    }
    text
}

/// The line break between two lines of a scalar: a space, or one line
/// break for each empty line between them.
fn push_break(text: &mut String, empty: usize) {
    if empty == 0 {
        text.push(' ');
    } else {
        text.push_str(&"\n".repeat(empty));
    }
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
/// may end its line, and the value is what the lines under it hold, as for
/// a key or a list entry with nothing after it; with none, it is empty.
fn tagged(cursor: &mut Cursor<'_>, context: Context) -> Result<Value, ParseError> {
    let tag = tag(cursor)?;
    let value = match context {
        Context::Entry(_) | Context::Key(_) => {
            cursor.skip_spaces();
            if at_line_end(cursor) {
                end_of_line(cursor)?;
                let block = nested(cursor, context, tag.span.end)?;
                cursor.set_pos(block.span().end);
                block
            } else {
                inline_value(cursor, context)?
            }
        }
        Context::Flow(column) => {
            skip_flow_space(cursor, column)?;
            inline_value(cursor, context)?
        }
    };
    retag(cursor, &tag, value)
}

/// A key written after a tag, read by `read` and checked against the tag.
fn tagged_key(
    cursor: &mut Cursor<'_>,
    read: fn(&mut Cursor<'_>) -> Result<String, ParseError>,
) -> Result<String, ParseError> {
    let tag = tag(cursor)?;
    cursor.skip_spaces();
    let start = cursor.pos();
    let text = read(cursor)?;
    let _: Value = retag(cursor, &tag, string(start..cursor.pos(), text.clone()))?;
    Ok(text)
}

/// Reads the tag at the cursor and the type it names: `!!int` and
/// `!<tag:yaml.org,2002:int>` both name `int`.
fn tag(cursor: &mut Cursor<'_>) -> Result<Tag, ParseError> {
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
    let span = start..cursor.pos();
    let Some(name) = named.and_then(|suffix| TAGS.into_iter().find(|known| *known == suffix))
    else {
        let message = format!("the tag `{written}` is not one archie reads");
        return Err(ParseError::new(&message, span));
    };
    Ok(Tag { name, span })
}

/// `value` read again under `tag`: a scalar's text gets the tag's type, and
/// a mapping or a list must be what the tag names. An empty value is read
/// as empty text, and an error about it points at the tag.
fn retag(cursor: &Cursor<'_>, tag: &Tag, value: Value) -> Result<Value, ParseError> {
    let name = tag.name;
    let scalar = match value {
        Value::Mapping(_) if name == "map" => return Ok(value),
        Value::Sequence(_) if name == "seq" => return Ok(value),
        Value::Mapping(mapping) => return Err(misfit("a mapping", name, mapping.span)),
        Value::Sequence(sequence) => return Err(misfit("a list", name, sequence.span)),
        Value::Scalar(scalar) => scalar,
    };
    let text = scalar_text(cursor, scalar.value, &scalar.span);
    let Some(typed) = typed(name, &text) else {
        if scalar.span.is_empty() {
            return Err(misfit("an empty value", name, tag.span.clone()));
        }
        return Err(misfit(&format!("`{text}`"), name, scalar.span));
    };
    Ok(Value::Scalar(Scalar {
        span: scalar.span,
        value: typed,
    }))
}

/// A scalar's text: a string's own, or the text any other value is written
/// as at `span`.
fn scalar_text(cursor: &Cursor<'_>, value: ScalarValue, span: &Range<usize>) -> String {
    match value {
        ScalarValue::String(text) => text,
        ScalarValue::Boolean(_)
        | ScalarValue::Date(_)
        | ScalarValue::DateTime(_)
        | ScalarValue::Float(_)
        | ScalarValue::Integer(_)
        | ScalarValue::LocalDateTime(_)
        | ScalarValue::NonFinite(_)
        | ScalarValue::Null
        | ScalarValue::Time(_) => folded(cursor.slice(span.clone())),
    }
}

/// The value `text` has under the scalar tag `!!name`, read by the rules of
/// an untagged value of that type, or `None` when it does not fit.
fn typed(name: &str, text: &str) -> Option<ScalarValue> {
    match (name, scalar::yaml(text)) {
        ("str", _) => Some(ScalarValue::String(text.to_owned())),
        ("float", ScalarValue::Integer(_)) => text.parse().ok().map(ScalarValue::Float),
        ("bool", fitting @ ScalarValue::Boolean(_))
        | ("float", fitting @ (ScalarValue::Float(_) | ScalarValue::NonFinite(_)))
        | ("int", fitting @ ScalarValue::Integer(_))
        | ("null", fitting @ ScalarValue::Null)
        | (
            "timestamp",
            fitting @ (ScalarValue::Date(_)
            | ScalarValue::DateTime(_)
            | ScalarValue::LocalDateTime(_)),
        ) => Some(fitting),
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

/// A double-quoted string. With no `context`, as for a key, it stays on one
/// line. A line break folds as in a plain scalar, and a `\` before it
/// removes it.
fn double_quoted(cursor: &mut Cursor<'_>, context: Option<Context>) -> Result<String, ParseError> {
    cursor.bump();
    let mut text = String::new();
    loop {
        let run = cursor.take_until(b"\"\\\n");
        if cursor.eat(b'"') {
            text.push_str(run);
            return Ok(text);
        }
        if cursor.eat(b'\\') {
            text.push_str(run);
            if matches!(cursor.peek(), Some(b'\r' | b'\n')) {
                let empty = line_break(cursor, context)?;
                text.push_str(&"\n".repeat(empty));
            } else {
                text.push(escape(cursor)?);
            }
        } else {
            text.push_str(run.trim_end_matches([' ', '\t', '\r']));
            let empty = line_break(cursor, context)?;
            push_break(&mut text, empty);
        }
    }
}

/// Reads the line break at the cursor inside a quoted string, the empty
/// lines after it and the indentation of the next line, and returns the
/// number of empty lines. The next line must be indented more than the
/// string's key or `-`, also inside a flow collection.
fn line_break(cursor: &mut Cursor<'_>, context: Option<Context>) -> Result<usize, ParseError> {
    let Some(within) = context else {
        return cursor.error("the string is not closed");
    };
    let mut empty = 0;
    loop {
        let _: bool = cursor.eat(b'\r');
        if !cursor.eat(b'\n') {
            return cursor.error("the string is not closed");
        }
        let line = cursor.rest_of_line();
        if !is_blank(line) {
            if let Some(column) = within.column()
                && leading_spaces(line) <= column
            {
                return cursor.error(
                    "a line inside a quoted string must be indented more than its key or `-`",
                );
            }
            cursor.skip_spaces();
            return Ok(empty);
        }
        cursor.set_pos(cursor.pos() + line.len());
        empty += 1;
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

/// A single-quoted string, on one line or over several as a double-quoted
/// one is.
fn single_quoted(cursor: &mut Cursor<'_>, context: Option<Context>) -> Result<String, ParseError> {
    cursor.bump();
    let mut text = String::new();
    loop {
        let run = cursor.take_until(b"'\n");
        if cursor.eat(b'\'') {
            text.push_str(run);
            // Inside single quotes, `''` is one quote.
            if !cursor.eat(b'\'') {
                return Ok(text);
            }
            text.push('\'');
        } else {
            text.push_str(run.trim_end_matches([' ', '\t', '\r']));
            let empty = line_break(cursor, context)?;
            push_break(&mut text, empty);
        }
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

/// Whether a list entry starts `offset` bytes after the cursor: a `-`
/// followed by a space, a tab or the end of the line.
fn is_dash(cursor: &Cursor<'_>, offset: usize) -> bool {
    cursor.peek_at(offset) == Some(b'-') && separated_at(cursor, offset + 1)
}

/// Whether the `?`, `:` or `-` at the cursor is an indicator rather than the
/// start of a plain scalar: a space, a tab or the end of the line follows
/// it, or inside a flow collection one of `,`, `[`, `]`, `{` or `}`.
fn is_indicator(cursor: &Cursor<'_>, in_flow: bool) -> bool {
    separated_at(cursor, 1)
        || (in_flow && matches!(cursor.peek_at(1), Some(b',' | b'[' | b']' | b'{' | b'}')))
}

/// Whether a space, a tab or the end of the line is `offset` bytes after the
/// cursor.
fn separated_at(cursor: &Cursor<'_>, offset: usize) -> bool {
    matches!(
        cursor.peek_at(offset),
        None | Some(b' ' | b'\t' | b'\n' | b'\r')
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
