//! The frontmatter block: finding it and reading it in its own syntax.

use core::{iter, mem};

use indexmap::IndexMap;

use crate::parser::ast::{Frontmatter, FrontmatterFormat, Mapping, Pair, Sequence, Value};
use crate::parser::error::ParseError;
use crate::parser::line::{Line, line_at};
use crate::parser::{toml, yaml};

/// The frontmatter `source` opens with, if any, and the offset its body
/// starts at.
pub fn parse(source: &str) -> Result<(Option<Frontmatter>, usize), ParseError> {
    let opening = line_at(source, 0);
    let format = match source[..opening.end].trim_end() {
        "+++" => FrontmatterFormat::Toml,
        "---" => FrontmatterFormat::Yaml,
        _ => return Ok((None, 0)),
    };
    let Some(closing) = closing(source, opening.next, format) else {
        return Ok((None, 0));
    };
    let content = opening.next..closing.start;
    let mapping = match format {
        FrontmatterFormat::Toml => toml::parse(source, content)?,
        FrontmatterFormat::Yaml => yaml::parse(source, content)?,
    };
    let frontmatter = Frontmatter {
        format,
        mapping,
        span: 0..closing.start + 3,
    };
    Ok((Some(frontmatter), closing.next))
}

/// Adds a pair to a mapping being read. A key written again joins the first
/// one: its value becomes a list of every value written for it.
pub fn insert(pairs: &mut IndexMap<String, Pair>, key: String, pair: Pair) {
    let Some(first) = pairs.get_mut(&key) else {
        pairs.insert(key, pair);
        return;
    };
    let span = first.value.span().clone();
    let mut items = into_items(mem::replace(&mut first.value, empty(span.start)));
    items.extend(into_items(pair.value));
    first.value = Value::Sequence(Sequence { items, span });
    first.repeats.push(pair.span);
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
    let end = pairs
        .values()
        .flat_map(|pair| iter::once(&pair.span).chain(&pair.repeats))
        .map(|span| span.end)
        .max();
    let span = pairs
        .first()
        .zip(end)
        .map_or(empty_at..empty_at, |((_, first), last_end)| {
            first.span.start..last_end
        });
    Mapping { pairs, span }
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
