//! The frontmatter block: finding it and reading it in its own syntax.

use core::ops::Range;
use core::{iter, mem};

use indexmap::IndexMap;
use indexmap::map::Entry;

use crate::parser::ast::{Frontmatter, FrontmatterFormat, Mapping, Pair, Sequence, Value};
use crate::parser::error::ParseError;
use crate::parser::line::{Line, line_at};
use crate::parser::{toml, yaml};

/// How deep the values the TOML and YAML readers build nest, counted
/// together: YAML block mappings 467 deep, each key indented under the one
/// before, overflow a 2 MB debug stack when they are read.
pub const VALUE_CAP: usize = 400;

/// The frontmatter `source` opens with at byte `start`, if any, and the
/// offset its body starts at.
pub fn parse(source: &str, start: usize) -> Result<(Option<Frontmatter>, usize), ParseError> {
    let opening = line_at(source, start);
    let format = match source[start..opening.end].trim_end() {
        "+++" => FrontmatterFormat::Toml,
        "---" => FrontmatterFormat::Yaml,
        _ => return Ok((None, start)),
    };
    let Some(closing) = closing(source, opening.next, format) else {
        return Ok((None, start));
    };
    let content = opening.next..closing.start;
    let mapping = match format {
        FrontmatterFormat::Toml => toml::parse(source, content)?,
        FrontmatterFormat::Yaml => yaml::parse(source, content)?,
    };
    let frontmatter = Frontmatter {
        format,
        mapping,
        span: start..closing.start + 3,
    };
    Ok((Some(frontmatter), closing.next))
}

/// The depth of a value opened at `span` inside `open` values, or an error
/// pointing at `span` when that passes `VALUE_CAP`.
pub fn nest(open: usize, span: Range<usize>) -> Result<usize, ParseError> {
    if open < VALUE_CAP {
        return Ok(open + 1);
    }
    let message = format!("the value nests deeper than {VALUE_CAP} levels");
    Err(ParseError::new(&message, span))
}

/// Adds a pair to a mapping being read. A key written again joins the first
/// one: its value becomes a list of every value written for it.
pub fn insert(pairs: &mut IndexMap<String, Pair>, key: String, pair: Pair) {
    match pairs.entry(key) {
        Entry::Vacant(slot) => {
            slot.insert(pair);
        }
        Entry::Occupied(slot) => {
            let first = slot.into_mut();
            first.repeats.push(pair.span);
            append(&mut first.value, pair.value);
        }
    }
}

/// Makes `first` a list of its own items followed by those of `value`. The
/// list keeps the range of `first`.
pub fn append(first: &mut Value, value: Value) {
    let span = first.span().clone();
    let mut items = into_items(mem::replace(first, empty(span.start)));
    items.extend(into_items(value));
    *first = Value::Sequence(Sequence { items, span });
}

/// A list gives its items; any other value gives itself.
fn into_items(value: Value) -> Vec<Value> {
    match value {
        Value::Sequence(sequence) => sequence.items,
        Value::Mapping(_) | Value::Scalar(_) => vec![value],
    }
}

/// A list with no items, at `at`.
const fn empty(at: usize) -> Value {
    Value::Sequence(Sequence {
        items: Vec::new(),
        span: at..at,
    })
}

/// The mapping holding `pairs`. Its span runs from the first key to the end
/// of the last value written; with no pairs it is empty, at `empty_at`.
pub fn finish(pairs: IndexMap<String, Pair>, empty_at: usize) -> Mapping {
    let span = extent(&pairs).unwrap_or(empty_at..empty_at);
    Mapping { pairs, span }
}

/// From the first key of `pairs` to the end of the last value written, or
/// `None` when there are no pairs.
pub fn extent(pairs: &IndexMap<String, Pair>) -> Option<Range<usize>> {
    let end = pairs
        .values()
        .flat_map(|pair| iter::once(&pair.span).chain(&pair.repeats))
        .map(|span| span.end)
        .max();
    pairs
        .first()
        .zip(end)
        .map(|((_, first), last_end)| first.span.start..last_end)
}

fn closing(source: &str, from: usize, format: FrontmatterFormat) -> Option<Line> {
    let mut start = from;
    while start < source.len() {
        let line = line_at(source, start);
        let text = source[line.start..line.end].trim_end();
        let closes = match format {
            FrontmatterFormat::Toml => text == "+++",
            FrontmatterFormat::Yaml => text == "---" || text == "...",
        };
        if closes {
            return Some(line);
        }
        start = line.next;
    }
    None
}
