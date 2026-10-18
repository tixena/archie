//! A document over any store: read in a form, and stored in archie's form.

use archie_core::forms::{Form, FormError, decode, encode, stored_text};
use archie_core::writer::WriteOptions;

use crate::{Precondition, Revision, Store, StoreError, StorePath};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DocumentError {
    #[error("{path}: {error}")]
    Form { path: StorePath, error: FormError },
    #[error(transparent)]
    Store(#[from] StoreError),
}

impl DocumentError {
    fn form<E>(path: &StorePath, error: E) -> Self
    where
        E: Into<FormError>,
    {
        Self::Form {
            path: path.clone(),
            error: error.into(),
        }
    }
}

/// The bytes of the document at `path` in `form`, its frontmatter in the
/// syntax `options` names when `form` is markdown.
///
/// # Errors
///
/// The store's error, or the form's, naming the path.
pub async fn get<S>(
    store: &S,
    path: &StorePath,
    form: Form,
    options: &WriteOptions,
) -> Result<Vec<u8>, DocumentError>
where
    S: Store + Sync,
{
    let stored = store.read(path).await?;
    let document =
        decode(&stored.bytes, Form::Markdown).map_err(|error| DocumentError::form(path, error))?;
    encode(&document, form, options).map_err(|error| DocumentError::form(path, error))
}

/// Stores at `path` archie's form of the document `bytes` hold in `form`, if
/// `precondition` holds, and gives the new revision.
///
/// # Errors
///
/// The form's error naming the path, or the store's.
pub async fn put<S>(
    store: &S,
    path: &StorePath,
    form: Form,
    bytes: &[u8],
    options: &WriteOptions,
    precondition: &Precondition,
) -> Result<Revision, DocumentError>
where
    S: Store + Sync,
{
    let text =
        stored_text(bytes, form, options).map_err(|error| DocumentError::form(path, error))?;
    Ok(store.write(path, text.as_bytes(), precondition).await?)
}

#[cfg(test)]
mod tests;
