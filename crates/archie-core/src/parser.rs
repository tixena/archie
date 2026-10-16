//! Markdown parsing: a source file into its document tree. The grammar it
//! follows is written in `GRAMMAR.md` at the root of this crate.

pub mod ast;
pub mod error;
pub mod extension;

mod block;
mod claim;
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

use crate::parser::ast::{Document, Kind, Mapping};
use crate::parser::error::ParseError;
use crate::parser::extension::{Context, Extension, Markdown, With};

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

/// Builds the tree, then offers every node to the extensions `T`.
fn parse_with_set<T>(source: &str, options: &ParseOptions) -> Result<Document<T>, ParseError>
where
    T: Extension,
{
    let (frontmatter, body_start) = frontmatter::parse(source)?;
    let mut document = Document {
        body: block::parse(source, body_start),
        frontmatter,
        kind: Kind(None),
        span: 0..source.len(),
    };
    let context = Context {
        source,
        strict: options.on_extension_error == OnExtensionError::Fail,
    };
    claim::document(&mut document, &context)?;
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
