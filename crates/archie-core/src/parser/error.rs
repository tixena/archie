//! The error a document fails to parse with.

use core::ops::Range;

use thiserror::Error;

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
