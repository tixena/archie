//! Why a source could not be read as a resource.

use core::ops::Range;

use thiserror::Error;

/// A source whose frontmatter archie could not read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuerySourceError {
    /// The frontmatter did not parse; `span` is where, in bytes.
    #[error("{message} (at byte {})", span.start)]
    Parse { message: String, span: Range<usize> },
}

/// A query that does not parse; `span` is where, in bytes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message} (at byte {})", span.start)]
#[non_exhaustive]
pub struct QueryError {
    pub message: String,
    pub span: Range<usize>,
}

impl QueryError {
    pub(crate) fn new(message: &str, span: Range<usize>) -> Self {
        Self {
            message: message.to_owned(),
            span,
        }
    }
}
