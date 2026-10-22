//! Files kept in a MongoDB collection, one record each.

pub mod record;

use alloc::collections::BTreeMap;
use core::fmt;

use archie_core::bundle::Missing;
use archie_core::forms::{Form, decode};
use archie_core::parser::ast::RawDocument;
use archie_core::writer::{self, WriteOptions};
use bson::oid::ObjectId;
use bson::{Document, RawDocumentBuf, doc};
use mongodb::error::{Error, ErrorKind, WriteFailure};
use mongodb::options::IndexOptions;
use mongodb::{Client, Collection, IndexModel};

use crate::mongo::record::{StoredDocument, from_record, revision_of, to_record};
use crate::{
    Capabilities, Precondition, Revision, Store, StoreError, StorePath, Stored, read_at_path,
    write_at_paths,
};

/// The code of a write that breaks a unique index.
const DUPLICATE_KEY: i32 = 11000;

/// The code of a command on a collection that does not exist.
const NAMESPACE_NOT_FOUND: i32 = 26;

/// The files of one collection of one database, each one record, with a
/// unique index on `path`.
pub struct MongoStore {
    collection: Collection<Document>,
}

impl MongoStore {
    /// Every record `filter` matches, read through `from_record`.
    ///
    /// # Errors
    ///
    /// `StoreError::Backend` with the driver's message, or for a record that
    /// does not read.
    pub async fn find(&self, filter: Document) -> Result<Vec<StoredDocument>, StoreError> {
        let mut records = self
            .collection
            .find(filter)
            .await
            .map_err(|error| backend(&error))?;
        let mut found = Vec::new();
        while records.advance().await.map_err(|error| backend(&error))? {
            found.push(from_record(record_of(records.current())?)?);
        }
        Ok(found)
    }

    /// Whether a unique index has `path` as its one key. A collection that
    /// does not exist has none.
    async fn has_path_index(&self) -> Result<bool, StoreError> {
        let mut indexes = match self.collection.list_indexes().await {
            Ok(indexes) => indexes,
            Err(error) if command_code(&error) == Some(NAMESPACE_NOT_FOUND) => return Ok(false),
            Err(error) => return Err(backend(&error)),
        };
        while indexes.advance().await.map_err(|error| backend(&error))? {
            let index = indexes
                .deserialize_current()
                .map_err(|error| backend(&error))?;
            let unique = index.options.and_then(|options| options.unique) == Some(true);
            if unique && index.keys.len() == 1 && index.keys.contains_key("path") {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Connects with `uri`, the connection string, and makes the unique index
    /// on `path` when the collection has none.
    ///
    /// # Errors
    ///
    /// `StoreError::Backend` with the driver's message.
    pub async fn open(uri: &str, database: &str, collection: &str) -> Result<Self, StoreError> {
        let client = Client::with_uri_str(uri)
            .await
            .map_err(|error| backend(&error))?;
        let store = Self {
            collection: client.database(database).collection(collection),
        };
        if !store.has_path_index().await? {
            let index = IndexModel::builder()
                .keys(doc! { "path": 1_i32 })
                .options(IndexOptions::builder().unique(true).build())
                .build();
            store
                .collection
                .create_index(index)
                .await
                .map_err(|error| backend(&error))?;
        }
        Ok(store)
    }

    /// The document at `path`, with its revision.
    ///
    /// # Errors
    ///
    /// `StoreError::NotFound` when no record is at `path`, or
    /// `StoreError::Backend` with the driver's message or for a record that
    /// does not read.
    pub async fn read_document(
        &self,
        path: &StorePath,
    ) -> Result<(RawDocument, Revision), StoreError> {
        let raw = self
            .collection
            .clone_with_type::<RawDocumentBuf>()
            .find_one(on_path(path)?)
            .await
            .map_err(|error| failed(path, &error))?
            .ok_or_else(|| StoreError::NotFound(path.clone()))?;
        let stored = from_record(record_of(&raw)?)?;
        Ok((stored.document, Revision(stored.revision)))
    }

    /// Writes `document` at `path` if `precondition` holds, and gives the new
    /// revision: the SHA-256 of the document in archie's form.
    ///
    /// # Errors
    ///
    /// `StoreError::Conflict` when `precondition` does not hold, or
    /// `StoreError::Backend` naming the path when the record nests too deep or
    /// the driver fails.
    pub async fn write_document(
        &self,
        path: &StorePath,
        document: &RawDocument,
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        let text = writer::write(document, &WriteOptions::default())
            .map_err(|error| failed(path, &error))?;
        let revision = revision_of(&text);
        let mut record = to_record(&StoredDocument {
            document: document.clone(),
            id: ObjectId::new(),
            path: path.as_str().to_owned(),
            revision: revision.0.clone(),
        })?;
        match precondition {
            Precondition::Absent => {
                self.collection.insert_one(record).await.map_err(|error| {
                    if duplicate_key(&error) {
                        StoreError::Conflict(path.clone())
                    } else {
                        failed(path, &error)
                    }
                })?;
            }
            Precondition::Matches(expected) => {
                // A replacement keeps the `_id` of the record it replaces.
                record.remove("_id");
                let replaced = self
                    .collection
                    .replace_one(on_revision(path, expected)?, record)
                    .await
                    .map_err(|error| failed(path, &error))?;
                if replaced.matched_count == 0 {
                    return Err(StoreError::Conflict(path.clone()));
                }
            }
            Precondition::None => {
                record.remove("_id");
                self.collection
                    .replace_one(on_path(path)?, record)
                    .upsert(true)
                    .await
                    .map_err(|error| failed(path, &error))?;
            }
        }
        Ok(revision)
    }
}

impl Store for MongoStore {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_preconditions: true,
        }
    }

    /// With `Precondition::Absent`, nothing is deleted: a record at `path` is
    /// `Conflict`, and none is `NotFound`.
    async fn delete(
        &self,
        path: &StorePath,
        precondition: &Precondition,
    ) -> Result<(), StoreError> {
        let (filter, unmatched) = match precondition {
            Precondition::Absent => {
                let held = self
                    .collection
                    .find_one(on_path(path)?)
                    .projection(doc! { "_id": 1_i32 })
                    .await
                    .map_err(|error| failed(path, &error))?;
                return Err(held.map_or_else(
                    || StoreError::NotFound(path.clone()),
                    |_| StoreError::Conflict(path.clone()),
                ));
            }
            Precondition::Matches(expected) => (
                on_revision(path, expected)?,
                StoreError::Conflict(path.clone()),
            ),
            Precondition::None => (on_path(path)?, StoreError::NotFound(path.clone())),
        };
        let deleted = self
            .collection
            .delete_one(filter)
            .await
            .map_err(|error| failed(path, &error))?;
        if deleted.deleted_count == 0 {
            Err(unmatched)
        } else {
            Ok(())
        }
    }

    async fn list(&self, prefix: Option<&StorePath>) -> Result<Vec<StorePath>, StoreError> {
        let filter = prefix.map_or_else(Document::new, |folder| {
            let pattern = format!("^{}/", escaped(folder.as_str()));
            StoredDocument::MONGO_FIELDS
                .path()
                .regex(&pattern, "")
                .into_document()
        });
        let mut records = self
            .collection
            .find(filter)
            .projection(doc! { "_id": 0_i32, "path": 1_i32 })
            .await
            .map_err(|error| backend(&error))?;
        let mut paths = Vec::new();
        while records.advance().await.map_err(|error| backend(&error))? {
            let record = records
                .deserialize_current()
                .map_err(|error| backend(&error))?;
            let path = record
                .get_str("path")
                .map_err(|error| StoreError::Backend(format!("a record's `path`: {error}")))?;
            paths.push(StorePath::new(path)?);
        }
        paths.sort_unstable();
        Ok(paths)
    }

    async fn read(&self, path: &StorePath) -> Result<Stored, StoreError> {
        let (document, revision) = self.read_document(path).await?;
        let text = writer::write(&document, &WriteOptions::default())
            .map_err(|error| failed(path, &error))?;
        Ok(Stored {
            bytes: text.into_bytes(),
            revision,
        })
    }

    async fn read_embedded(
        &self,
        _document: &StorePath,
        file: &StorePath,
    ) -> Result<Vec<u8>, StoreError> {
        read_at_path(self, file).await
    }

    async fn write(
        &self,
        path: &StorePath,
        bytes: &[u8],
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        let bundle =
            decode(bytes, Form::Markdown, Missing::Keep).map_err(|error| failed(path, &error))?;
        self.write_document(path, bundle.document(), precondition)
            .await
    }

    async fn write_with_embedded(
        &self,
        path: &StorePath,
        text: &str,
        files: &BTreeMap<StorePath, Vec<u8>>,
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        write_at_paths(self, path, text, files, precondition).await
    }
}

fn backend(error: &dyn fmt::Display) -> StoreError {
    StoreError::Backend(error.to_string())
}

fn command_code(error: &Error) -> Option<i32> {
    if let ErrorKind::Command(command) = error.kind.as_ref() {
        Some(command.code)
    } else {
        None
    }
}

fn duplicate_key(error: &Error) -> bool {
    matches!(
        error.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(failure)) if failure.code == DUPLICATE_KEY
    )
}

/// `text` with a `\` before each character a regular expression reads as
/// more than itself.
fn escaped(text: &str) -> String {
    let mut pattern = String::with_capacity(text.len());
    for character in text.chars() {
        if "\\^$.|?*+()[]{}".contains(character) {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern
}

fn failed(path: &StorePath, error: &dyn fmt::Display) -> StoreError {
    StoreError::Backend(format!("{path}: {error}"))
}

/// `raw` as a `Document`, converted without serde: the driver's serde read of
/// a deep record overflows a 2 MB debug stack.
fn record_of(raw: &bson::RawDocument) -> Result<Document, StoreError> {
    Document::try_from(raw).map_err(|error| backend(&error))
}

/// The typed filter on `path`.
fn on_path(path: &StorePath) -> Result<Document, StoreError> {
    let filter = StoredDocument::MONGO_FIELDS
        .path()
        .eq(path.as_str().to_owned())
        .map_err(|error| failed(path, &error))?;
    Ok(filter.into_document())
}

/// The typed filter on `path` and `revision`.
fn on_revision(path: &StorePath, revision: &Revision) -> Result<Document, StoreError> {
    let paths = StoredDocument::MONGO_FIELDS;
    let filter = paths
        .path()
        .eq(path.as_str().to_owned())
        .and_then(|on_path| Ok(on_path.and(paths.revision().eq(revision.0.clone())?)))
        .map_err(|error| failed(path, &error))?;
    Ok(filter.into_document())
}

#[cfg(test)]
#[cfg(store_tests)]
mod tests;
