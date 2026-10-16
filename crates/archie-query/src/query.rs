//! The frontmatter query language: `fm.KEY OP VALUE`, `and`, `or`, `not`,
//! parentheses.
//!
//! Operators: `==`, `!=`, `<`, `<=`, `>`, `>=`, `has`. A missing field makes
//! every operator false except `!=`. Values of different types are never
//! equal, except that a string written as a date or datetime compares as one.

mod eval;
mod lexer;
mod parse;

use crate::error::{QueryError, QuerySourceError};
use crate::resource::Resource;

/// A parsed query, ready to test resources against.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    expr: parse::Expr,
}

impl Query {
    /// Whether `resource` satisfies the query.
    #[must_use]
    pub fn matches(&self, resource: &Resource) -> bool {
        eval::eval(&self.expr, resource)
    }

    /// Whether the document `source` satisfies the query. A document with no
    /// frontmatter has no fields.
    ///
    /// # Errors
    ///
    /// Returns an error when archie cannot read the frontmatter.
    pub fn matches_source(&self, source: &str) -> Result<bool, QuerySourceError> {
        let resource = Resource::from_source(source)?.unwrap_or(Resource {
            fields: Vec::new(),
            kind: None,
        });
        Ok(self.matches(&resource))
    }

    /// Parses `text` into a query.
    ///
    /// # Errors
    ///
    /// Returns an error naming the byte span where the query stops making sense.
    pub fn parse(text: &str) -> Result<Self, QueryError> {
        Ok(Self {
            expr: parse::parse(text)?,
        })
    }
}

#[cfg(test)]
mod tests;
