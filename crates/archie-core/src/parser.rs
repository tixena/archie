//! Markdown parsing: a source file into its document tree. The grammar it
//! follows is written in `GRAMMAR.md` at the root of this crate.

pub mod ast;
pub mod error;

mod block;
mod cursor;
mod frontmatter;
mod inline;
mod lexer;
mod line;
mod scalar;
mod table;
mod toml;
mod yaml;

use core::ops::Range;

use crate::parser::ast::{Document, Extension, Mapping, Plain};
use crate::parser::error::ParseError;

/// Parses a markdown file into its document tree.
///
/// # Errors
///
/// Returns an error when the frontmatter cannot be read. The body never
/// fails: what is not recognized stays as text.
pub fn parse(source: &str) -> Result<Document, ParseError> {
    parse_with::<Plain>(source)
}

/// Parses a markdown file into its document tree, asking the extension `E`
/// for the subtype of each fenced code block.
///
/// # Errors
///
/// Returns an error when the frontmatter cannot be read, or when `E` claims
/// a code block and fails to read it.
pub fn parse_with<E>(source: &str) -> Result<Document<E>, ParseError>
where
    E: Extension,
{
    let (frontmatter, body_start) = frontmatter::parse(source)?;
    Ok(Document {
        body: block::parse(source, body_start)?,
        frontmatter,
        span: 0..source.len(),
    })
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

/// Reads the YAML written at `range` of `source` into a mapping, the way
/// YAML frontmatter is read. Spans count from the start of `source`.
///
/// # Errors
///
/// Returns an error when the text is not YAML that archie reads.
pub fn read_yaml(source: &str, range: Range<usize>) -> Result<Mapping, ParseError> {
    yaml::parse(source, range)
}

#[cfg(test)]
mod tests;
