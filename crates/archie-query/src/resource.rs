//! A markdown file seen as a resource: its frontmatter fields and its type.

use crate::adapters::ast;
use crate::error::QuerySourceError;
use crate::field::Field;

/// The frontmatter of one document, with its `type` when that is a string.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Resource {
    pub fields: Vec<(String, Field)>,
    pub kind: Option<String>,
}

impl Resource {
    /// The field at `path`, following nested maps (`["phone", "work"]`).
    #[must_use]
    pub fn field(&self, path: &[&str]) -> Option<&Field> {
        let (first, rest) = path.split_first()?;
        let mut current = lookup(&self.fields, first)?;
        for key in rest {
            let Field::Map(pairs) = current else {
                return None;
            };
            current = lookup(pairs, key)?;
        }
        Some(current)
    }

    /// Reads `source` as a resource. `Ok(None)` when it has no frontmatter.
    ///
    /// # Errors
    ///
    /// Returns an error when archie cannot read the frontmatter.
    pub fn from_source(source: &str) -> Result<Option<Self>, QuerySourceError> {
        Ok(ast::frontmatter_fields(source)?.map(|fields| {
            let kind = match lookup(&fields, "type") {
                Some(Field::Str(text)) => Some(text.clone()),
                _ => None,
            };
            Self { fields, kind }
        }))
    }
}

fn lookup<'fields>(pairs: &'fields [(String, Field)], key: &str) -> Option<&'fields Field> {
    pairs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

#[cfg(test)]
mod tests;
