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
