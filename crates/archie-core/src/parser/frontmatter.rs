//! The frontmatter block: finding it and reading it in its own syntax.

use core::ops::Range;

use indexmap::IndexMap;

use crate::parser::ast::{Frontmatter, FrontmatterFormat, Mapping, Pair};
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

/// Adds a pair to a mapping being read. A key may be written only once.
pub fn insert(
    pairs: &mut IndexMap<String, Pair>,
    key: String,
    key_span: Range<usize>,
    pair: Pair,
) -> Result<(), ParseError> {
    if pairs.contains_key(&key) {
        let message = format!("the key `{key}` is written twice");
        return Err(ParseError::new(&message, key_span));
    }
    pairs.insert(key, pair);
    Ok(())
}

/// The mapping holding `pairs`. With no pairs its span is empty, at
/// `empty_at`.
pub fn finish(pairs: IndexMap<String, Pair>, empty_at: usize) -> Mapping {
    let span = pairs
        .first()
        .zip(pairs.last())
        .map_or(empty_at..empty_at, |((_, first), (_, last))| {
            first.span.start..last.span.end
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
