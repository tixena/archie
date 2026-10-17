//! YAML frontmatter: block and flow collections, block scalars, tags and
//! plain scalars into a mapping.

use core::ops::Range;

use indexmap::IndexMap;

use crate::parser::ast::{Mapping, Pair, Scalar, ScalarValue, Sequence, Value};
use crate::parser::cursor::Cursor;
use crate::parser::error::ParseError;
use crate::parser::frontmatter::{Stop, caught, finish, insert, nest};
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

/// One pair as read: kept under its key, or dropped because its key is a
/// list or a mapping, which TOML cannot hold.
enum Read {
    Dropped,
    Kept(String, Pair),
}

impl Read {
    /// The pair from `start` to the end of `value`, kept under `name`, or
    /// dropped when it has none.
    fn new(name: Option<String>, start: usize, value: Value) -> Self {
        let Some(key) = name else {
            return Self::Dropped;
        };
        let pair = Pair {
            repeats: Vec::new(),
            span: start..value.span().end,
            value,
        };
        Self::Kept(key, pair)
    }
}

/// A tag as written: the type it names, `None` when archie does not read
/// it, and where it is written.
struct Tag {
    name: Option<&'static str>,
    span: Range<usize>,
}

pub fn parse(source: &str, content: Range<usize>) -> Result<Mapping, ParseError> {
    let empty_at = content.start;
    let mut cursor = Cursor::new(source, content);
    let Some(indent) = next_line(&mut cursor) else {
        return Ok(finish(IndexMap::new(), empty_at));
    };
    let line_start = cursor.pos();
    cursor.set_pos(line_start + indent);
    no_tab_before_block(&mut cursor, 0)?;
    // A tab after the indentation separates a flow node from it.
    cursor.skip_spaces();
    let root = cursor.pos();
    match cursor.peek() {
        Some(b'{') if !starts_mapping(&mut cursor, 0) => {
            let mapping = flow_mapping(&mut cursor, None, 0)?;
            end_of_line(&mut cursor)?;
            if next_line(&mut cursor).is_some() {
                return cursor.error("expected the end of the frontmatter after the mapping");
            }
            Ok(mapping)
        }
        Some(b'[') if !starts_mapping(&mut cursor, 0) => {
            let read = flow_sequence(&mut cursor, None, 0);
            match caught(&mut cursor, root, read, skip_collection) {
                Ok(list) => only_value(&mut cursor, root, list.ok()),
                Err(_) => Err(not_a_mapping(&to_last_text(&cursor, root))),
            }
        }
        Some(_) | None => {
            cursor.set_pos(line_start);
            let mapping = match block_mapping(&mut cursor, indent, 0) {
                Ok(mapping) => mapping,
                Err(error) => {
                    cursor.set_pos(root);
                    return lone_value(&mut cursor, indent).unwrap_or_else(|| Err(error.into()));
                }
            };
            if next_line(&mut cursor).is_some() {
                return cursor.error("this line is indented less than the first key");
            }
            Ok(mapping)
        }
    }
}

/// The frontmatter whose content is the value at the cursor, which fills its
/// line as a scalar or a list does: see [`only_value`]. `None` when the line
/// holds a key and does not read or raises a drop.
fn lone_value(cursor: &mut Cursor<'_>, indent: usize) -> Option<Result<Mapping, ParseError>> {
    let start = cursor.pos();
    let is_list = is_dash(cursor, 0);
    let is_value = is_list || !holds_key(cursor);
    let read = if is_list {
        block_sequence(cursor, start - indent, indent, 0)
    } else {
        inline_value(cursor, Context::Key(indent), 0).and_then(|value| {
            end_of_line(cursor)?;
            Ok(value)
        })
    };
    match caught(cursor, start, read, |inner| skip_block(inner, indent)) {
        Ok(Ok(value)) => Some(only_value(cursor, start, Some(value))),
        Ok(Err(_)) => is_value.then(|| only_value(cursor, start, None)),
        Err(_) => is_value.then(|| Err(not_a_mapping(&to_last_text(cursor, start)))),
    }
}

/// The frontmatter whose content is one value, read from `start` up to the
/// cursor: `value`, or `None` when it raised a drop. With nothing after it,
/// that is an empty mapping spanning the value, as no key holds it.
fn only_value(
    cursor: &mut Cursor<'_>,
    start: usize,
    value: Option<Value>,
) -> Result<Mapping, ParseError> {
    let span = value.map_or_else(|| to_last_text(cursor, start), |read| read.span().clone());
    if end_of_line(cursor).is_err() || next_line(cursor).is_some() {
        return Err(not_a_mapping(&span));
    }
    Ok(Mapping {
        pairs: IndexMap::new(),
        span,
    })
}

/// Whether the line at the cursor holds a key: it starts a mapping, or a
/// `key:` follows an anchor, a tag or an indicator on it.
fn holds_key(cursor: &mut Cursor<'_>) -> bool {
    starts_mapping(cursor, 0) || holds_pair(uncommented(cursor.rest_of_line()).trim_end())
}

/// From `start` to the end of the frontmatter's last line of text.
fn to_last_text(cursor: &Cursor<'_>, start: usize) -> Range<usize> {
    let text = cursor
        .slice(start..cursor.end())
        .trim_end_matches([' ', '\t', '\r', '\n']);
    start..start + text.len()
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

/// The pairs written at exactly `indent`, starting at the cursor's line: a
/// mapping `depth` values deep.
fn block_mapping(cursor: &mut Cursor<'_>, indent: usize, depth: usize) -> Result<Mapping, Stop> {
    let empty_at = cursor.pos() + indent;
    let mut pairs = IndexMap::new();
    mapping_lines(cursor, indent, depth, &mut pairs)?;
    Ok(finish(pairs, empty_at))
}

/// The pairs on the lines from the cursor on that are written at exactly
/// `indent`, of a mapping `depth` values deep.
fn mapping_lines(
    cursor: &mut Cursor<'_>,
    indent: usize,
    depth: usize,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), Stop> {
    while let Some(found) = next_line(cursor) {
        if found < indent {
            break;
        }
        if found > indent {
            return cursor.error("this line is indented more than the keys above it");
        }
        cursor.set_pos(cursor.pos() + indent);
        if cursor.peek() == Some(b'\t') {
            return Err(tab_indent(cursor.pos()).into());
        }
        if is_dash(cursor, 0) {
            return cursor.error("expected a key, found a list entry");
        }
        let start = cursor.pos();
        let read = entry(cursor, indent, depth, pairs);
        skip_dropped(cursor, start, indent, read)?;
    }
    Ok(())
}

/// `read`, the pair of a block mapping that starts at `start` with its key
/// at column `indent`; when it raised a drop, the cursor moves past the pair,
/// which adds nothing.
fn skip_dropped(
    cursor: &mut Cursor<'_>,
    start: usize,
    indent: usize,
    read: Result<(), Stop>,
) -> Result<(), Stop> {
    let _: Result<(), ParseError> = caught(cursor, start, read, |inner| skip_block(inner, indent))?;
    Ok(())
}

/// Adds the pair `read` gives to `pairs`; a dropped pair adds nothing.
fn add(pairs: &mut IndexMap<String, Pair>, read: Read) {
    if let Read::Kept(name, pair) = read {
        insert(pairs, name, pair);
    }
}

/// A mapping `depth` values deep whose first key is written on a list
/// entry's line, at `column`; its other keys follow on lines indented to that
/// column.
fn entry_mapping(cursor: &mut Cursor<'_>, column: usize, depth: usize) -> Result<Mapping, Stop> {
    let empty_at = cursor.pos();
    let mut pairs = IndexMap::new();
    let read = entry(cursor, column, depth, &mut pairs);
    skip_dropped(cursor, empty_at, column, read)?;
    mapping_lines(cursor, column, depth, &mut pairs)?;
    Ok(finish(pairs, empty_at))
}

/// One pair of a block mapping whose keys are at column `indent`, inside
/// `open` values: a `key:` and its value, a `:` and its value with the empty
/// key, or an explicit key after `?`. A drop leaves `pairs` as it was.
fn entry(
    cursor: &mut Cursor<'_>,
    indent: usize,
    open: usize,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), Stop> {
    if is_explicit_key(cursor) {
        return explicit_entry(cursor, indent, open, pairs);
    }
    let start = cursor.pos();
    let name = if is_empty_key(cursor, false) {
        Some(String::new())
    } else {
        key_or_collection(cursor, Context::Key(indent), open, key)?
    };
    cursor.skip_spaces();
    if !cursor.eat(b':') {
        return cursor.error("expected `:` after the key");
    }
    if !separated_at(cursor, 0) {
        return cursor.error("expected a space after the `:`");
    }
    let value = indicated_value(cursor, indent, open)?;
    add(pairs, Read::new(name, start, value));
    Ok(())
}

/// A pair whose key follows a `?` at column `indent`, inside `open` values;
/// the `?` counts as a level of its own. Its value follows a `:` at that
/// column on the next line with content; with none, it is null at the end of
/// the key. Both lines are read as a list entry's `-` line is.
fn explicit_entry(
    cursor: &mut Cursor<'_>,
    indent: usize,
    open: usize,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), Stop> {
    let start = cursor.pos();
    let depth = nest(open, start..start + 1)?;
    let key = compact_value(cursor, start - indent, indent, Context::Key(indent), depth)?;
    let key_end = key.span().end;
    let name = explicit_key(cursor, key);
    let value = if next_line(cursor) == Some(indent)
        && cursor.peek_at(indent) == Some(b':')
        && separated_at(cursor, indent + 1)
    {
        let line_start = cursor.pos();
        compact_value(cursor, line_start, indent, Context::Key(indent), depth)?
    } else {
        null(key_end)
    };
    add(pairs, Read::new(name, start, value));
    Ok(())
}

/// Moves the cursor from the start of a pair whose key is at column `indent`
/// past it: the rest of its line, then the lines indented more and the list
/// entries at that column, and after `?` the line of its `:` value.
fn skip_block(cursor: &mut Cursor<'_>, indent: usize) {
    let mut value_line = is_explicit_key(cursor);
    loop {
        let _: &str = cursor.take_until(b"\n");
        cursor.bump();
        let Some(found) = next_line(cursor) else {
            return;
        };
        let at_key = found == indent;
        let colon = value_line
            && at_key
            && cursor.peek_at(found) == Some(b':')
            && separated_at(cursor, found + 1);
        if found < indent || (at_key && !colon && !is_dash(cursor, found)) {
            return;
        }
        value_line &= !colon;
    }
}

/// The text of an explicit key, or `None` for a list or a mapping.
fn explicit_key(cursor: &Cursor<'_>, key: Value) -> Option<String> {
    match key {
        Value::Scalar(scalar) => Some(scalar_text(cursor, scalar.value, &scalar.span)),
        Value::Mapping(_) | Value::Sequence(_) => None,
    }
}

/// The key at the cursor, read by `text`, or `None` for a key that is a list
/// or a mapping, tagged or not, which is read as a value written in `context`
/// inside `open` values.
fn key_or_collection<F>(
    cursor: &mut Cursor<'_>,
    context: Context,
    open: usize,
    text: F,
) -> Result<Option<String>, Stop>
where
    F: FnOnce(&mut Cursor<'_>) -> Result<String, Stop>,
{
    if !is_collection_key(cursor) {
        return text(cursor).map(Some);
    }
    let _: Value = inline_value(cursor, context, open)?;
    Ok(None)
}

/// Whether a `[` or a `{` starts the node at the cursor, after a tag or not.
fn is_collection_key(cursor: &mut Cursor<'_>) -> bool {
    let start = cursor.pos();
    skip_tag(cursor);
    let is_collection = matches!(cursor.peek(), Some(b'[' | b'{'));
    cursor.set_pos(start);
    is_collection
}

/// Moves the cursor past the tag at it, whether archie reads it or not, and
/// the spaces after it.
fn skip_tag(cursor: &mut Cursor<'_>) {
    if cursor.peek() == Some(b'!') {
        pass_tag(cursor);
        cursor.skip_spaces();
    }
}

/// The value after the `:` of a key at column `indent`, with the cursor
/// right after the `:`: the rest of the line, what the lines under it hold,
/// or null right after the `:`.
fn indicated_value(cursor: &mut Cursor<'_>, indent: usize, open: usize) -> Result<Value, Stop> {
    let after = cursor.pos();
    cursor.skip_spaces();
    if at_line_end(cursor) {
        end_of_line(cursor)?;
        return nested(cursor, Context::Key(indent), after, open);
    }
    let value = inline_value(cursor, Context::Key(indent), open)?;
    end_of_line(cursor)?;
    Ok(value)
}

fn key(cursor: &mut Cursor<'_>) -> Result<String, Stop> {
    match cursor.peek() {
        Some(b'"') => double_quoted(cursor, None).map_err(Stop::Fail),
        Some(b'\'') => single_quoted(cursor, None).map_err(Stop::Fail),
        Some(b'!') => tagged_key(cursor, false, key),
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

/// Whether the cursor, inside `open` values, is on a pair that opens a
/// mapping: an explicit key, or an empty key or a `key:` after a tag or not,
/// whose key is plain, quoted or a flow collection. On a tab it is not.
fn starts_mapping(cursor: &mut Cursor<'_>, open: usize) -> bool {
    if is_explicit_key(cursor) {
        return true;
    }
    let start = cursor.pos();
    skip_tag(cursor);
    let is_mapping = is_empty_key(cursor, false)
        || match cursor.peek() {
            Some(b'"' | b'\'') => {
                let is_key = key(cursor).is_ok();
                cursor.skip_spaces();
                is_key && cursor.peek() == Some(b':')
            }
            // Reading the key checks the indentation of its lines.
            Some(b'[' | b'{') => {
                let is_key = passes_collection(cursor, open);
                cursor.skip_spaces();
                is_key && cursor.peek() == Some(b':') && separated_at(cursor, 1)
            }
            Some(b'\t' | b'|' | b'>' | b'&' | b'*' | b'!') => false,
            Some(_) | None => plain_key_length(cursor.rest_of_line()).is_some(),
        };
    cursor.set_pos(start);
    is_mapping
}

/// Whether the flow collection at the cursor, inside `open` values, reads,
/// and moves the cursor past it. One that raises a drop is passed over
/// unread.
fn passes_collection(cursor: &mut Cursor<'_>, open: usize) -> bool {
    let start = cursor.pos();
    match inline_value(cursor, Context::Flow(None), open) {
        Ok(_) => true,
        Err(Stop::Drop(_)) => {
            cursor.set_pos(start);
            skip_collection(cursor);
            true
        }
        Err(Stop::Fail(_)) => false,
    }
}

/// Whether the cursor is on a `?` that starts an explicit key: one followed
/// by a space, a tab or the end of the line.
fn is_explicit_key(cursor: &Cursor<'_>) -> bool {
    cursor.peek() == Some(b'?') && separated_at(cursor, 1)
}

/// The value of a key, a list entry or a tag with nothing after it on its
/// line, inside `open` values: what the lines indented more than it hold, a
/// list at a key's own column, or null.
fn nested(
    cursor: &mut Cursor<'_>,
    context: Context,
    null_at: usize,
    open: usize,
) -> Result<Value, Stop> {
    let Some((found, is_list)) = under(cursor, context) else {
        return Ok(null(null_at));
    };
    let line_start = cursor.pos();
    let at = line_start + found;
    if is_list {
        let depth = nest(open, at..at + 1)?;
        return block_sequence(cursor, line_start, found, depth);
    }
    cursor.set_pos(at);
    no_tab_before_block(cursor, open)?;
    if starts_mapping(cursor, open) {
        let depth = nest(open, at..at + 1)?;
        cursor.set_pos(line_start);
        return Ok(Value::Mapping(block_mapping(cursor, found, depth)?));
    }
    // A tab after the indentation separates a flow node from it.
    cursor.skip_spaces();
    line_value(cursor, context, open)
}

/// The indentation of the line that holds the value of a key, a list entry
/// or a tag with nothing after it on its line, and whether a list entry
/// starts that line, with the cursor at its start; `None` when the value is
/// null.
fn under(cursor: &mut Cursor<'_>, context: Context) -> Option<(usize, bool)> {
    let found = next_line(cursor)?;
    let is_list = is_dash(cursor, found);
    let compact = is_list && matches!(context, Context::Key(column) if column == found);
    if !compact && context.column().is_some_and(|column| found <= column) {
        return None;
    }
    Some((found, is_list))
}

/// Refuses the tab at the cursor, after the indentation of a line inside
/// `open` values, when a key, an explicit key or a list entry follows it.
fn no_tab_before_block(cursor: &mut Cursor<'_>, open: usize) -> Result<(), ParseError> {
    let tab = cursor.pos();
    if cursor.peek() != Some(b'\t') {
        return Ok(());
    }
    cursor.skip_spaces();
    let starts_block = is_dash(cursor, 0) || starts_mapping(cursor, open);
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

/// A block list `depth` values deep whose first `-` is at column `indent` of
/// the line starting at `line_start`, and whose other entries are the lines
/// below with a `-` at that column.
fn block_sequence(
    cursor: &mut Cursor<'_>,
    line_start: usize,
    indent: usize,
    depth: usize,
) -> Result<Value, Stop> {
    let start = line_start + indent;
    let mut entry_line = line_start;
    let mut items = Vec::new();
    loop {
        items.push(compact_value(
            cursor,
            entry_line,
            indent,
            Context::Entry(indent),
            depth,
        )?);
        let Some(found) = next_line(cursor) else {
            break;
        };
        if found > indent {
            return cursor.error("this line is indented more than the list above it");
        }
        if found == indent && cursor.peek_at(found) == Some(b'\t') {
            return Err(tab_indent(cursor.pos() + found).into());
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
/// starting at `line_start`, written in `context` inside `open` values. A `-`
/// after it starts a list at its own column, and a `key:` or a `?` there a
/// mapping, unless a tab comes before them: a tab is never indentation.
fn compact_value(
    cursor: &mut Cursor<'_>,
    line_start: usize,
    indent: usize,
    context: Context,
    open: usize,
) -> Result<Value, Stop> {
    cursor.set_pos(line_start + indent + 1);
    let after_indicator = cursor.pos();
    cursor.skip_spaces();
    let at = cursor.pos();
    let column = at - line_start;
    if at_line_end(cursor) {
        end_of_line(cursor)?;
        return nested(cursor, context, after_indicator, open);
    }
    let indented = !cursor.slice(after_indicator..at).contains('\t');
    if indented && is_dash(cursor, 0) {
        let depth = nest(open, at..at + 1)?;
        return block_sequence(cursor, line_start, column, depth);
    }
    if indented && starts_mapping(cursor, open) {
        let depth = nest(open, at..at + 1)?;
        return Ok(Value::Mapping(entry_mapping(cursor, column, depth)?));
    }
    line_value(cursor, context, open)
}

/// The value that starts a line of a block or follows its `-`, `?` or `:`,
/// inside `open` values, up to the end of that line.
fn line_value(cursor: &mut Cursor<'_>, context: Context, open: usize) -> Result<Value, Stop> {
    let value = inline_value(cursor, context, open)?;
    end_of_line(cursor)?;
    Ok(value)
}

/// A value that starts at the cursor, inside `open` values. It ends on its
/// own line, except a flow collection, which may run over several, and a
/// block scalar, which takes the lines under it; either way the cursor stops
/// after its last text.
fn inline_value(cursor: &mut Cursor<'_>, context: Context, open: usize) -> Result<Value, Stop> {
    let start = cursor.pos();
    match cursor.peek() {
        Some(b'[') => flow_sequence(cursor, context.column(), nest(open, start..start + 1)?),
        Some(b'{') => flow_mapping(cursor, context.column(), nest(open, start..start + 1)?)
            .map(Value::Mapping),
        Some(b'"') => {
            let text = double_quoted(cursor, Some(context))?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'\'') => {
            let text = single_quoted(cursor, Some(context))?;
            Ok(string(start..cursor.pos(), text))
        }
        Some(b'|' | b'>') => match context {
            Context::Entry(indent) | Context::Key(indent) => {
                block_scalar(cursor, indent).map_err(Stop::Fail)
            }
            Context::Flow(_) => plain(cursor, context).map_err(Stop::Fail),
        },
        Some(b'!') => tagged(cursor, context, open),
        Some(b'&' | b'*') => anchor(cursor),
        Some(_) | None => plain(cursor, context).map_err(Stop::Fail),
    }
}

/// A flow list `depth` values deep whose lines are indented more than
/// `column`, the column of its key or `-`; none at the root.
fn flow_sequence(
    cursor: &mut Cursor<'_>,
    column: Option<usize>,
    depth: usize,
) -> Result<Value, Stop> {
    let start = cursor.pos();
    cursor.bump();
    let mut items = Vec::new();
    loop {
        skip_flow_space(cursor, column)?;
        if cursor.eat(b']') {
            break;
        }
        no_empty_entry(cursor)?;
        items.push(flow_item(cursor, column, depth)?);
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

/// One entry of a flow list, inside `open` values: a value, or a pair, which
/// makes a mapping of one pair. A list or a mapping is read once, and is the
/// key of a pair when a `:` follows it on its line.
fn flow_item(cursor: &mut Cursor<'_>, column: Option<usize>, open: usize) -> Result<Value, Stop> {
    let start = cursor.pos();
    if is_collection_key(cursor) {
        let item = match inline_value(cursor, Context::Flow(column), open) {
            Err(Stop::Drop(error)) => {
                // Passed over, to see whether a `:` makes it a key.
                cursor.set_pos(start);
                skip_tag(cursor);
                skip_collection(cursor);
                Err(Stop::Drop(error))
            }
            read @ (Ok(_) | Err(Stop::Fail(_))) => read,
        };
        if matches!(item, Err(Stop::Fail(_))) || !colon_follows(cursor) {
            return item;
        }
        return one_pair(cursor, column, open, start, false);
    }
    if is_flow_pair(cursor) {
        return one_pair(cursor, column, open, start, true);
    }
    inline_value(cursor, Context::Flow(column), open)
}

/// The mapping of the one pair of a flow list that starts at `start`, inside
/// `open` values. With `read_key`, its key is read here; otherwise the cursor
/// is after a key that is a list or a mapping.
fn one_pair(
    cursor: &mut Cursor<'_>,
    column: Option<usize>,
    open: usize,
    start: usize,
    read_key: bool,
) -> Result<Value, Stop> {
    let depth = nest(open, start..start + 1)?;
    let name = if read_key {
        flow_pair_key(cursor, column, true, depth)
    } else {
        Ok(None)
    };
    let mut pairs = IndexMap::new();
    let read = match name {
        Ok(key) => flow_pair(cursor, column, depth, start, key, &mut pairs),
        Err(stop) => Err(stop),
    };
    let _: Result<(), ParseError> = caught(cursor, start, read, skip_flow_entry)?;
    Ok(Value::Mapping(finish(pairs, start)))
}

/// Whether a `:` follows the cursor on its line, after spaces or tabs.
fn colon_follows(cursor: &Cursor<'_>) -> bool {
    cursor
        .rest_of_line()
        .trim_start_matches([' ', '\t'])
        .starts_with(':')
}

/// Whether the flow entry at the cursor is a pair: an explicit key, an empty
/// key or a key followed by `:`.
fn is_flow_pair(cursor: &mut Cursor<'_>) -> bool {
    if is_flow_explicit_key(cursor) || is_empty_key(cursor, true) {
        return true;
    }
    let start = cursor.pos();
    // A key's tag that archie does not read raises a drop after the key.
    let is_key = !matches!(flow_key(cursor, None), Err(Stop::Fail(_)));
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

/// A flow mapping `depth` values deep whose lines are indented more than
/// `column`, the column of its key or `-`; none at the root. A drop inside a
/// pair drops that pair.
fn flow_mapping(
    cursor: &mut Cursor<'_>,
    column: Option<usize>,
    depth: usize,
) -> Result<Mapping, Stop> {
    let start = cursor.pos();
    cursor.bump();
    let mut pairs = IndexMap::new();
    loop {
        skip_flow_space(cursor, column)?;
        if cursor.eat(b'}') {
            break;
        }
        no_empty_entry(cursor)?;
        let entry = cursor.pos();
        let read = match flow_pair_key(cursor, column, false, depth) {
            Ok(name) => flow_pair(cursor, column, depth, entry, name, &mut pairs),
            Err(stop) => Err(stop),
        };
        let _: Result<(), ParseError> = caught(cursor, entry, read, skip_flow_entry)?;
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

/// The rest of a flow pair that starts at `start`, inside `open` values, after
/// its key `name`. Line breaks and comments may come before its `:`; with no
/// `:`, or with nothing after it, the value is null.
fn flow_pair(
    cursor: &mut Cursor<'_>,
    column: Option<usize>,
    open: usize,
    start: usize,
    name: Option<String>,
    pairs: &mut IndexMap<String, Pair>,
) -> Result<(), Stop> {
    let key_end = cursor.pos();
    skip_flow_space(cursor, column)?;
    let value = if cursor.eat(b':') {
        let after_colon = cursor.pos();
        skip_flow_space(cursor, column)?;
        if matches!(cursor.peek(), Some(b',' | b'}' | b']')) {
            null(after_colon)
        } else {
            inline_value(cursor, Context::Flow(column), open)?
        }
    } else {
        null(key_end)
    };
    add(pairs, Read::new(name, start, value));
    Ok(())
}

/// Moves the cursor from the start of a flow entry that is dropped to its
/// end: the first `,`, `]` or `}` outside every bracket the entry opens.
/// Quoted scalars, tags and comments are passed over whole.
fn skip_flow_entry(cursor: &mut Cursor<'_>) {
    let mut depth = 0_usize;
    // Whether a node may start here, as a quoted scalar or a tag may.
    let mut node_start = true;
    let mut spaced = true;
    loop {
        let Some(byte) = cursor.peek() else {
            return;
        };
        match byte {
            b',' | b']' | b'}' if depth == 0 => return,
            b'"' | b'\'' if node_start => {
                let context = Some(Context::Flow(None));
                let _: Result<String, ParseError> = if byte == b'"' {
                    double_quoted(cursor, context)
                } else {
                    single_quoted(cursor, context)
                };
                node_start = false;
                spaced = false;
                continue;
            }
            b'!' if node_start => {
                pass_tag(cursor);
                spaced = false;
                continue;
            }
            b'#' if spaced => {
                let _: &str = cursor.take_until(b"\n");
                continue;
            }
            b'[' | b'{' | b',' => node_start = true,
            b']' | b'}' => {
                depth -= 1;
                node_start = false;
            }
            b':' | b'?' if is_indicator(cursor, true) => node_start = true,
            b' ' | b'\t' | b'\r' | b'\n' => {}
            _ => node_start = false,
        }
        if matches!(byte, b'[' | b'{') {
            depth += 1;
        }
        spaced = matches!(byte, b' ' | b'\t' | b'\r' | b'\n');
        cursor.bump();
    }
}

/// Moves the cursor past the flow collection at it, entry by entry as
/// [`skip_flow_entry`] passes over one.
fn skip_collection(cursor: &mut Cursor<'_>) {
    cursor.bump();
    loop {
        skip_flow_entry(cursor);
        if !cursor.eat(b',') {
            cursor.bump();
            return;
        }
    }
}

/// The key of a flow pair inside `open` values, or `None` for a list or a
/// mapping; empty after a `?` with none or before a `:` that starts the pair.
/// It goes on over lines in a flow mapping or after a `?`, else stays on one.
fn flow_pair_key(
    cursor: &mut Cursor<'_>,
    column: Option<usize>,
    in_list: bool,
    open: usize,
) -> Result<Option<String>, Stop> {
    let explicit = is_flow_explicit_key(cursor);
    if explicit {
        cursor.bump();
        skip_flow_space(cursor, column)?;
    }
    if is_empty_key(cursor, true)
        || (explicit && matches!(cursor.peek(), None | Some(b',' | b']' | b'}')))
    {
        return Ok(Some(String::new()));
    }
    let context = Context::Flow(column);
    key_or_collection(cursor, context, open, |inner| {
        flow_key(inner, (explicit || !in_list).then_some(context))
    })
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

/// A key inside a flow collection. A plain key ends at a `:` followed by a
/// space or a flow indicator, at a flow indicator, or at a comment. With a
/// `context`, as inside a flow mapping, a quoted key may go on over several
/// lines as a quoted value does, and a plain key that runs to the end of its
/// line goes on over the lines below as a `PLAIN` does, up to that end.
fn flow_key(cursor: &mut Cursor<'_>, context: Option<Context>) -> Result<String, Stop> {
    match cursor.peek() {
        Some(b'"') => double_quoted(cursor, context).map_err(Stop::Fail),
        Some(b'\'') => single_quoted(cursor, context).map_err(Stop::Fail),
        Some(b'!') => tagged_key(cursor, true, |inner| flow_key(inner, context)),
        Some(b'&' | b'*') => anchor(cursor),
        Some(_) | None => {
            no_indicator(cursor, true, "key")?;
            let start = cursor.pos();
            let line = cursor.rest_of_line();
            let key = flow_key_part(line);
            let text = key.trim_end();
            if text.is_empty() {
                return cursor.error("expected a key");
            }
            let first_end = start + text.len();
            let end = match context {
                Some(within) if key.len() == line.len() => {
                    continued_end(cursor, first_end, within, |content| {
                        Some(flow_key_part(content))
                    })
                }
                Some(_) | None => first_end,
            };
            cursor.set_pos(end);
            Ok(folded(cursor.slice(start..end)))
        }
    }
}

/// The start of `line` that a plain key inside a flow collection may hold:
/// up to a `:` followed by a space or a flow indicator, a flow indicator, or
/// a comment.
fn flow_key_part(line: &str) -> &str {
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
    &line[..length]
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
    let written = folded(cursor.slice(start..end));
    Ok(Value::Scalar(Scalar {
        span: start..end,
        value: scalar::yaml(&written),
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
/// last line below that goes on with it. Such a line holds no `key:` and
/// starts with no flow indicator; a comment or a flow indicator after its
/// text makes it the last.
fn plain_end(cursor: &Cursor<'_>, first_end: usize, within: Context) -> usize {
    continued_end(cursor, first_end, within, |content| {
        let bounded = plain_part(content, within);
        (!holds_pair(bounded.trim_end())).then_some(bounded)
    })
}

/// Where a plain scalar or key whose first line ends at `first_end` ends:
/// after the last line below that goes on with it. Such a line comes after
/// nothing but blank lines, is indented more than the column of `within`,
/// does not start with a comment, and has text in the part of it that `part`
/// gives, which is none when the line cannot go on with it; a part shorter
/// than the line makes it the last.
fn continued_end<F>(cursor: &Cursor<'_>, first_end: usize, within: Context, part: F) -> usize
where
    F: Fn(&str) -> Option<&str>,
{
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
            let shallow = within
                .column()
                .is_some_and(|column| leading_spaces(line) <= column);
            if shallow || content.starts_with('#') {
                break;
            }
            let Some(bounded) = part(content) else {
                break;
            };
            let text = bounded.trim_end();
            if text.is_empty() {
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

/// Whether a plain scalar's text holds a `:` followed by a space or a tab, or
/// ends with `:`, as a key does.
fn holds_pair(text: &str) -> bool {
    text.contains(": ") || text.contains(":\t") || text.ends_with(':')
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

/// A value written after a tag, inside `open` values, read again under it.
/// In block context a tag may end its line, and the value is what the lines
/// under it hold, as for a key or a list entry with nothing after it; with
/// none, it is empty. A collection counts its tag as a level of its own, and
/// a value has at most one tag.
fn tagged(cursor: &mut Cursor<'_>, context: Context, open: usize) -> Result<Value, Stop> {
    let tag = tag(cursor);
    let under = match context {
        Context::Entry(_) | Context::Key(_) => {
            cursor.skip_spaces();
            at_line_end(cursor)
        }
        Context::Flow(column) => {
            skip_flow_space(cursor, column)?;
            false
        }
    };
    let depth = if under {
        end_of_line(cursor)?;
        depth_under(cursor, context, &tag, open)?
    } else {
        tagged_depth(cursor, &tag, open)?
    };
    let name = known(cursor, &tag)?;
    let value = if under {
        let block = nested(cursor, context, tag.span.end, depth)?;
        to_line_end(cursor);
        block
    } else {
        inline_value(cursor, context, depth)?
    };
    retag(cursor, name, &tag.span, value).map_err(Stop::Fail)
}

/// Moves the cursor from the start of a line back onto the line ending
/// before it, where the block read over the lines above ends; the span of a
/// mapping whose every pair is dropped does not reach that far.
fn to_line_end(cursor: &mut Cursor<'_>) {
    let at = cursor.pos();
    if at > 0 && cursor.slice(at - 1..at) == "\n" {
        cursor.set_pos(at - 1);
    }
}

/// The depth inside `open` values that the node after `tag`, at the cursor,
/// is read at: a flow collection counts the tag as a level of its own. A
/// second tag on the node is refused.
fn tagged_depth(cursor: &Cursor<'_>, tag: &Tag, open: usize) -> Result<usize, Stop> {
    match cursor.peek() {
        Some(b'!') => Err(second_tag(cursor.pos()).into()),
        Some(b'[' | b'{') => tag_level(tag, open),
        Some(_) | None => Ok(open),
    }
}

/// [`tagged_depth`] for the node that `nested` reads under a tag that ends
/// its line, with the cursor at the start of the next line: a block
/// collection there counts the tag as a level of its own too.
fn depth_under(
    cursor: &mut Cursor<'_>,
    context: Context,
    tag: &Tag,
    open: usize,
) -> Result<usize, Stop> {
    let start = cursor.pos();
    let depth = match under(cursor, context) {
        None => Ok(open),
        Some((_, true)) => tag_level(tag, open),
        Some((found, false)) => {
            cursor.set_pos(cursor.pos() + found);
            no_tab_before_block(cursor, open)?;
            if starts_mapping(cursor, open) {
                tag_level(tag, open)
            } else {
                cursor.skip_spaces();
                tagged_depth(cursor, tag, open)
            }
        }
    };
    cursor.set_pos(start);
    depth
}

/// The depth inside `open` values that a collection after `tag` opens in:
/// the tag counts as a level of its own.
fn tag_level(tag: &Tag, open: usize) -> Result<usize, Stop> {
    let at = tag.span.start;
    nest(open, at..at + 1)
}

/// A tag at `at` on a node that already has one.
fn second_tag(at: usize) -> ParseError {
    ParseError::new("a value has at most one tag", at..at + 1)
}

/// A key written after a tag, read by `read` and checked against the tag. A
/// `:` right after the tag leaves the key empty, as `is_empty_key` reads it
/// inside a flow collection or not, and a second tag is refused.
fn tagged_key<F>(cursor: &mut Cursor<'_>, in_flow: bool, read: F) -> Result<String, Stop>
where
    F: FnOnce(&mut Cursor<'_>) -> Result<String, Stop>,
{
    let tag = tag(cursor);
    cursor.skip_spaces();
    let start = cursor.pos();
    if cursor.peek() == Some(b'!') {
        return Err(second_tag(start).into());
    }
    let text = if is_empty_key(cursor, in_flow) {
        String::new()
    } else {
        read(cursor)?
    };
    let name = known(cursor, &tag)?;
    let _: Value = retag(
        cursor,
        name,
        &tag.span,
        string(start..cursor.pos(), text.clone()),
    )?;
    Ok(text)
}

/// Reads the tag at the cursor and the type it names: `!!int` and
/// `!<tag:yaml.org,2002:int>` both name `int`, and `!point` none.
fn tag(cursor: &mut Cursor<'_>) -> Tag {
    let start = cursor.pos();
    pass_tag(cursor);
    let written = cursor.slice(start..cursor.pos());
    let named = written.strip_prefix("!!").or_else(|| {
        written
            .strip_prefix("!<tag:yaml.org,2002:")?
            .strip_suffix('>')
    });
    Tag {
        name: named.and_then(|suffix| TAGS.into_iter().find(|known| *known == suffix)),
        span: start..cursor.pos(),
    }
}

/// Moves the cursor past the tag at it: a tag written in full ends at its
/// `>`, any other at a space, a tab, a line ending or a flow indicator.
fn pass_tag(cursor: &mut Cursor<'_>) {
    if cursor.rest().starts_with("!<") {
        let _: &str = cursor.take_until(b" \t\r\n>");
        let _: bool = cursor.eat(b'>');
    } else {
        let _: &str = cursor.take_until(b" \t\r\n,[]{}");
    }
}

/// The type `tag` names, or a drop when archie does not read it.
fn known(cursor: &Cursor<'_>, tag: &Tag) -> Result<&'static str, Stop> {
    tag.name.ok_or_else(|| {
        let written = cursor.slice(tag.span.clone());
        let message = format!("the tag `{written}` is not one archie reads");
        Stop::Drop(ParseError::new(&message, tag.span.clone()))
    })
}

/// `value` read again under the tag `!!name` at `at`: a scalar's text gets
/// the tag's type, and a mapping or a list must be what the tag names. An
/// empty value is read as empty text, and an error about it points at `at`.
fn retag(
    cursor: &Cursor<'_>,
    name: &str,
    at: &Range<usize>,
    value: Value,
) -> Result<Value, ParseError> {
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
            return Err(misfit("an empty value", name, at.clone()));
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
        ("float", ScalarValue::Integer(_) | ScalarValue::String(_)) => scalar::decimal_float(text),
        ("int", oversized @ ScalarValue::String(_)) if scalar::is_oversized_integer(text) => {
            Some(oversized)
        }
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
fn anchor<T>(cursor: &Cursor<'_>) -> Result<T, Stop> {
    let at = cursor.pos();
    Err(ParseError::new("anchors and aliases are not read", at..at + 1).into())
}

/// A double-quoted string. With no `context`, as for a key held to its line,
/// it stays on one line. A line break folds as in a plain scalar, and a `\` before it
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
