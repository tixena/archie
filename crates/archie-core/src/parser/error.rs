//! The errors a document fails to parse or to be rebuilt with.

use core::ops::Range;

use thiserror::Error;

use crate::writer::WriteError;

/// Why a document could not be parsed, and where.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message} (at byte {})", span.start)]
#[non_exhaustive]
pub struct ParseError {
    pub message: String,
    pub span: Range<usize>,
}

impl ParseError {
    #[must_use]
    pub fn new(message: &str, span: Range<usize>) -> Self {
        Self {
            message: message.to_owned(),
            span,
        }
    }
}

/// Why a stored tree could not be built again: it could not be written, or
/// the written text could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RebuildError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Write(#[from] WriteError),
}
