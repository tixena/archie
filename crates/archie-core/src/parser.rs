//! Markdown parsing: a source file into its document tree. The grammar it
//! follows is written in `GRAMMAR.md` at the root of this crate.

pub mod ast;
pub mod error;
pub mod extension;

mod block;
mod claim;
mod cursor;
pub(crate) mod definition;
pub(crate) mod entity;
mod frontmatter;
pub(crate) mod html;
pub(crate) mod inline;
pub(crate) mod lexer;
pub(crate) mod line;
pub(crate) mod scalar;
pub(crate) mod table;
mod toml;
mod yaml;

use core::ops::Range;

use serde::ser::Error as _;
use serde::{Deserialize as _, Serialize as _};

use crate::parser::ast::{Document, Kind, Mapping, Value};
use crate::parser::error::{ParseError, RebuildError};
use crate::parser::extension::{Context, Extension, Frame, Markdown, With};
use crate::writer;

/// What happens when an extension claims a construct and cannot read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum OnExtensionError {
    /// The extension's error fails the parse.
    Fail,
    /// The construct stays the plain node archie built.
    #[default]
    Fallback,
}

/// How a file is parsed.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ParseOptions {
    pub on_extension_error: OnExtensionError,
}

impl ParseOptions {
    /// A parse that an extension's error fails.
    #[must_use]
    pub const fn strict() -> Self {
        Self {
            on_extension_error: OnExtensionError::Fail,
        }
    }
}

/// Parses a markdown file into its document tree, with archie's markdown
/// extension alone.
///
/// # Errors
///
/// Returns an error when the frontmatter cannot be read. The body never
/// fails: what is not recognized stays as text.
pub fn parse(source: &str) -> Result<Document, ParseError> {
    parse_with_set::<Markdown>(source, &ParseOptions::default())
}

/// Parses a markdown file into its document tree, with archie's markdown
/// extension first and `E` after it.
///
/// # Errors
///
/// Returns an error when the frontmatter cannot be read, or, in a strict
/// parse, when an extension claims a construct and cannot read it.
pub fn parse_with<E>(source: &str, options: &ParseOptions) -> Result<Document<With<E>>, ParseError>
where
    E: Extension,
{
    parse_with_set::<With<E>>(source, options)
}

/// Builds the extension nodes of `document` again, for a document read from
/// storage that keeps only the plain tree, such as the JSON `to_json` writes
/// and `from_json` reads.
///
/// The plain tree is written in archie's form and parsed again with `E`, so
/// the spans of the document returned count from that written text.
///
/// # Errors
///
/// Returns `RebuildError::Write`, naming the key, when the `Toml` args of a
/// code block hold a null, which TOML cannot hold, and `RebuildError::Parse`
/// when the parse fails, as `parse_with` does.
pub fn rebuild_with<E>(
    document: &Document<With<E>>,
    options: &ParseOptions,
) -> Result<Document<With<E>>, RebuildError>
where
    E: Extension,
{
    let text = writer::write_for_rebuild(document)?;
    Ok(parse_with::<E>(&text, options)?)
}

/// Reads a document from the JSON `to_json` writes, at any depth archie reads.
///
/// # Errors
///
/// Returns `serde_json`'s error when the text is not such a document.
pub fn from_json<E>(text: &str) -> Result<Document<E>, serde_json::Error>
where
    E: Extension,
{
    let mut reader = serde_json::Deserializer::from_str(text);
    reader.disable_recursion_limit();
    let document = Document::deserialize(serde_stacker::Deserializer::new(&mut reader))?;
    reader.end()?;
    Ok(document)
}

/// Writes `document` as JSON, at any depth archie reads. No node writes its
/// `kind`; `rebuild_with` builds it again.
///
/// # Errors
///
/// Returns `serde_json`'s error when a value cannot be written as JSON.
pub fn to_json<E>(document: &Document<E>) -> Result<String, serde_json::Error>
where
    E: Extension,
{
    let mut text = Vec::new();
    document.serialize(serde_stacker::Serializer::new(
        &mut serde_json::Serializer::new(&mut text),
    ))?;
    String::from_utf8(text).map_err(serde_json::Error::custom)
}

/// Builds the tree, then offers every node to the extensions `T`.
fn parse_with_set<T>(source: &str, options: &ParseOptions) -> Result<Document<T>, ParseError>
where
    T: Extension,
{
    let start = if source.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };
    let (frontmatter, body_start) = frontmatter::parse(source, start)?;
    let (body, _, cells) = block::parse(source, body_start);
    let mut document = Document {
        body,
        frontmatter,
        kind: Kind(None),
        span: 0..source.len(),
    };
    let context = Context {
        column: 0,
        depth: 0,
        frame: Frame::NONE,
        lines: None,
        source,
        strict: options.on_extension_error == OnExtensionError::Fail,
    };
    claim::document(&mut document, &context, &cells)?;
    Ok(document)
}

/// Reads the TOML written at `range` of `source` into a mapping, the way
/// TOML frontmatter is read. Spans count from the start of `source`.
///
/// # Errors
///
/// Returns an error when the text is not TOML that archie reads.
pub fn read_toml(source: &str, range: Range<usize>) -> Result<Mapping, ParseError> {
    toml::parse(source, range)
}

/// Reads the TOML written on `lines`, ranges of `source`, as consecutive
/// lines of one text, the way `read_toml` reads a range. Spans count from
/// the start of `source`.
///
/// # Errors
///
/// Returns an error when the text is not TOML that archie reads.
pub fn read_toml_lines(source: &str, lines: &[Range<usize>]) -> Result<Mapping, ParseError> {
    read_lines(source, lines, toml::parse)
}

/// Reads the YAML written at `range` of `source` into a mapping, the way
/// YAML frontmatter is read. Spans count from the start of `source`.
///
/// # Errors
///
/// Returns an error when the text is not YAML that archie reads.
pub fn read_yaml(source: &str, range: Range<usize>) -> Result<Mapping, ParseError> {
    yaml::parse(source, range)
}

/// Reads the YAML written on `lines`, ranges of `source`, as consecutive
/// lines of one text, the way `read_yaml` reads a range. Spans count from
/// the start of `source`.
///
/// # Errors
///
/// Returns an error when the text is not YAML that archie reads.
pub fn read_yaml_lines(source: &str, lines: &[Range<usize>]) -> Result<Mapping, ParseError> {
    read_lines(source, lines, yaml::parse)
}

/// Reads `lines`, ranges of `source` joined by `\n`, with `read`, and places
/// every span of the mapping or of the error back in `source`.
fn read_lines(
    source: &str,
    lines: &[Range<usize>],
    read: fn(&str, Range<usize>) -> Result<Mapping, ParseError>,
) -> Result<Mapping, ParseError> {
    let mut starts = Vec::with_capacity(lines.len());
    let mut text = String::new();
    for line in lines {
        if !starts.is_empty() {
            text.push('\n');
        }
        starts.push(text.len());
        text.push_str(&source[line.clone()]);
    }
    let place = |span: &Range<usize>| {
        let at = |offset: usize| {
            let index = starts
                .partition_point(|&start| start <= offset)
                .saturating_sub(1);
            lines
                .get(index)
                .zip(starts.get(index))
                .map_or(offset, |(line, &start)| line.start + (offset - start))
        };
        at(span.start)..at(span.end)
    };
    match read(&text, 0..text.len()) {
        Ok(mut mapping) => {
            place_mapping(&mut mapping, &place);
            Ok(mapping)
        }
        Err(mut error) => {
            error.span = place(&error.span);
            Err(error)
        }
    }
}

fn place_mapping<F>(mapping: &mut Mapping, place: &F)
where
    F: Fn(&Range<usize>) -> Range<usize>,
{
    mapping.span = place(&mapping.span);
    for pair in mapping.pairs.values_mut() {
        pair.span = place(&pair.span);
        for repeat in &mut pair.repeats {
            *repeat = place(repeat);
        }
        place_value(&mut pair.value, place);
    }
}

fn place_value<F>(value: &mut Value, place: &F)
where
    F: Fn(&Range<usize>) -> Range<usize>,
{
    match value {
        Value::Mapping(mapping) => place_mapping(mapping, place),
        Value::Scalar(scalar) => scalar.span = place(&scalar.span),
        Value::Sequence(sequence) => {
            sequence.span = place(&sequence.span);
            for item in &mut sequence.items {
                place_value(item, place);
            }
        }
    }
}

#[cfg(test)]
mod tests;
