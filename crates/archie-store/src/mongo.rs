//! Files kept in a MongoDB collection, one record each, and the files they
//! embed kept as objects of a `MinIO` bucket, each named by a record of a
//! second collection.

pub mod record;

use alloc::collections::BTreeMap;
use core::fmt;
use core::str::from_utf8;
use core::time::Duration;

use archie_core::bundle::{Missing, embedded_paths, images_mut, media_type};
use archie_core::forms::{Form, decode};
use archie_core::parser::ast::RawDocument;
use archie_core::parser::parse;
use archie_core::writer::{self, WriteOptions};
use bson::oid::ObjectId;
use bson::{Document, RawDocumentBuf, doc};
use mongodb::error::{Error, ErrorKind, WriteFailure};
use mongodb::options::IndexOptions;
use mongodb::{Client, Collection, IndexModel};
use os_shim::bucket::MinioBucket;
use serde::Deserialize;
use tokio::time;

use crate::document::{is_note, parent_of};
use crate::file;
use crate::mongo::record::stored_object_schema::{Update, WriteError};
use crate::mongo::record::{
    Claim, StoredDocument, StoredObject, from_record, revision_of, to_record,
};
use crate::object::ObjectStore;
use crate::{Capabilities, Precondition, Revision, Store, StoreError, StorePath, Stored};

/// How many times a save tries to claim an object whose record a deletion
/// holds.
const CLAIM_TRIES: u32 = 20;

/// How long a save waits between two of those tries.
const CLAIM_WAIT: Duration = Duration::from_millis(50);

/// The code of a write that breaks a unique index.
const DUPLICATE_KEY: i32 = 11000;

/// The code of a command on a collection that does not exist.
const NAMESPACE_NOT_FOUND: i32 = 26;

/// A file a save writes: its path beside the document, its path in the
/// bundle, its bytes, their SHA-256, the path of its object, and the alt text
/// of the first embed that names it.
struct Embedded<'save> {
    bytes: &'save [u8],
    description: String,
    hash: String,
    name: &'save str,
    object: StorePath,
    path: &'save StorePath,
}

/// The `files` of a document's record, read alone.
#[derive(Deserialize)]
struct HeldFiles {
    #[serde(default)]
    files: BTreeMap<String, String>,
}

/// The files of one collection of one database, each one record, with a
/// unique index on `path`.
///
/// Each file they embed is the object `files/<sha256>` of `files`, named by
/// one record of `<collection>_files`, which has an index on
/// `embedded_by.path`. An object nothing embeds is deleted, then its record.
pub struct MongoStore {
    collection: Collection<Document>,
    files: ObjectStore<MinioBucket>,
    objects: Collection<Document>,
}

impl MongoStore {
    /// Claims the object of `file` for the document at `path`, trying again
    /// while a deletion holds its record.
    async fn claim(&self, path: &StorePath, file: &Embedded<'_>) -> Result<(), StoreError> {
        let mut tries = 1;
        while !self.try_claim(path, file).await? {
            if tries == CLAIM_TRIES {
                return Err(StoreError::Backend(format!(
                    "{}: a deletion of its object still runs",
                    file.path
                )));
            }
            tries += 1;
            time::sleep(CLAIM_WAIT).await;
        }
        Ok(())
    }

    /// Deletes the object whose bytes have the SHA-256 `hash`, then its
    /// record, when nothing embeds it and no other deletion holds it.
    async fn delete_object(&self, hash: &str) -> Result<(), StoreError> {
        if self.mark(hash).await? {
            self.remove_object(hash).await?;
            self.remove_record(hash).await?;
        }
        Ok(())
    }

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

    /// The `files` of the record at `path`, read alone: none when no record
    /// is there.
    async fn held_files(&self, path: &StorePath) -> Result<BTreeMap<String, String>, StoreError> {
        let Some(record) = self
            .collection
            .find_one(on_path(path)?)
            .projection(doc! { "_id": 0_i32, "files": 1_i32 })
            .await
            .map_err(|error| failed(path, &error))?
        else {
            return Ok(BTreeMap::new());
        };
        let held: HeldFiles =
            bson::deserialize_from_document(record).map_err(|error| failed(path, &error))?;
        Ok(held.files)
    }

    /// Marks the record of the object whose bytes have the SHA-256 `hash` as
    /// held by a deletion, only while nothing embeds it and no other deletion
    /// holds it: `false` when no record was marked.
    async fn mark(&self, hash: &str) -> Result<bool, StoreError> {
        let object = object_path(hash)?;
        let (filter, update) = marking(hash).map_err(|error| failed(&object, &error))?;
        let marked = self
            .objects
            .update_one(filter, update)
            .await
            .map_err(|error| failed(&object, &error))?;
        Ok(marked.matched_count == 1)
    }

    /// Connects with `uri`, the connection string, keeps each embedded file
    /// in `files`, and makes the unique index on `path` and the index on
    /// `embedded_by.path` of `<collection>_files` where they are missing.
    ///
    /// # Errors
    ///
    /// `StoreError::Backend` with the driver's message.
    pub async fn open(
        uri: &str,
        database: &str,
        collection: &str,
        files: ObjectStore<MinioBucket>,
    ) -> Result<Self, StoreError> {
        let client = Client::with_uri_str(uri)
            .await
            .map_err(|error| backend(&error))?;
        let store = Self {
            collection: client.database(database).collection(collection),
            files,
            objects: client
                .database(database)
                .collection(&format!("{collection}_files")),
        };
        let indexes = [
            (&store.collection, "path", true),
            (&store.objects, "embedded_by.path", false),
        ];
        for (indexed, key, unique) in indexes {
            if !has_index(indexed, key, unique).await? {
                let mut keys = Document::new();
                keys.insert(key, 1_i32);
                let index = IndexModel::builder()
                    .keys(keys)
                    .options(IndexOptions::builder().unique(unique).build())
                    .build();
                indexed
                    .create_index(index)
                    .await
                    .map_err(|error| backend(&error))?;
            }
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

    /// The bytes of the file at `path`, where no document is: the object that
    /// the record of a document claiming the path names.
    async fn read_file(&self, path: &StorePath) -> Result<Vec<u8>, StoreError> {
        let claimed = Claim::MONGO_FIELDS
            .path()
            .eq(path.as_str().to_owned())
            .map_err(|error| failed(path, &error))?;
        let records = StoredObject::find(
            &self.objects,
            StoredObject::MONGO_FIELDS.embedded_by().nest(claimed),
        )
        .await
        .map_err(|error| failed(path, &error))?;
        for record in records {
            for claim in record
                .embedded_by
                .iter()
                .filter(|claim| claim.path == path.as_str())
            {
                match self
                    .read_embedded(&StorePath::new(&claim.document)?, path)
                    .await
                {
                    Err(StoreError::NotFound(_)) => {}
                    read => return read,
                }
            }
        }
        Err(StoreError::NotFound(path.clone()))
    }

    /// Takes back each claim of the document at `path` that `files`, the
    /// SHA-256 of each file it embeds by its path in the bundle, does not make,
    /// and gives the SHA-256 of each object it took a claim from.
    async fn release(
        &self,
        path: &StorePath,
        files: &BTreeMap<String, String>,
    ) -> Result<Vec<String>, StoreError> {
        let of_document = Claim::MONGO_FIELDS
            .document()
            .eq(path.as_str().to_owned())
            .map_err(|error| failed(path, &error))?;
        let records = StoredObject::find(
            &self.objects,
            StoredObject::MONGO_FIELDS.embedded_by().nest(of_document),
        )
        .await
        .map_err(|error| failed(path, &error))?;
        let mut released = Vec::new();
        for record in records {
            let of_path = record
                .embedded_by
                .iter()
                .filter(|claim| claim.document == path.as_str());
            let mut taken = false;
            for claim in of_path {
                let made = name_in(path, &claim.path).and_then(|name| files.get(name));
                if made != Some(&record.id) {
                    let (filter, update) =
                        releasing(&record.id, claim).map_err(|error| failed(path, &error))?;
                    self.objects
                        .update_one(filter, update)
                        .await
                        .map_err(|error| failed(path, &error))?;
                    taken = true;
                }
            }
            if taken {
                released.push(record.id);
            }
        }
        Ok(released)
    }

    /// Deletes the record of the document at `path` if `precondition` holds,
    /// as `Store::delete` says.
    async fn remove_document(
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

    /// Deletes the object whose bytes have the SHA-256 `hash` from the bucket.
    async fn remove_object(&self, hash: &str) -> Result<(), StoreError> {
        self.files
            .delete(&object_path(hash)?, &Precondition::None)
            .await
    }

    /// Deletes the record of the object whose bytes have the SHA-256 `hash`,
    /// only while a deletion holds it.
    async fn remove_record(&self, hash: &str) -> Result<(), StoreError> {
        let object = object_path(hash)?;
        let paths = StoredObject::MONGO_FIELDS;
        let held = paths
            .id()
            .eq(hash.to_owned())
            .and_then(|on_id| Ok(on_id.and(paths.deleting().eq(true)?)))
            .map_err(|error| failed(&object, &error))?;
        self.objects
            .delete_one(held.into_document())
            .await
            .map_err(|error| failed(&object, &error))?;
        Ok(())
    }

    /// Writes `document` at `path`, with `files`, the SHA-256 of each file it
    /// embeds by its path in the bundle, if `precondition` holds, and gives
    /// the new revision.
    async fn save(
        &self,
        path: &StorePath,
        document: &RawDocument,
        files: &BTreeMap<String, String>,
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        let text = writer::write(document, &WriteOptions::default())
            .map_err(|error| failed(path, &error))?;
        let revision = revision_of(&text);
        let mut record = to_record(&StoredDocument {
            document: document.clone(),
            files: files.clone(),
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

    /// Each entry of the `files` of the record at `path` whose file `document`
    /// embeds, an embedded note's own embeds included, each note read from its
    /// object.
    async fn still_embedded(
        &self,
        path: &StorePath,
        document: &RawDocument,
    ) -> Result<BTreeMap<String, String>, StoreError> {
        let held = self.held_files(path).await?;
        let mut kept = BTreeMap::new();
        let mut pending = vec![(String::new(), document.clone())];
        while let Some((folder, text)) = pending.pop() {
            let named = embedded_paths(&text, &folder).map_err(|error| failed(path, &error))?;
            for (_, name) in named {
                if kept.contains_key(&name) {
                    continue;
                }
                let file = match path.beside(&name) {
                    Ok(file) => file,
                    // `document::get` reads no file at a path that is not a store path.
                    Err(StoreError::InvalidPath(_)) => continue,
                    Err(error) => return Err(error),
                };
                let Some(hash) = held.get(&name) else {
                    continue;
                };
                if is_note(&name) {
                    match self.files.read(&object_path(hash)?).await {
                        Ok(object) => {
                            let note =
                                from_utf8(&object.bytes).map_err(|error| failed(&file, &error))?;
                            pending.push((parent_of(&name).to_owned(), parse(note).document));
                        }
                        Err(StoreError::NotFound(_)) => {}
                        Err(error) => return Err(error),
                    }
                }
                kept.insert(name, hash.clone());
            }
        }
        Ok(kept)
    }

    /// One try at claiming the object of `file` for the document at `path`:
    /// its record gains the claim, and is made with the object's metadata
    /// where none is. `false` while a deletion holds the record.
    async fn try_claim(&self, path: &StorePath, file: &Embedded<'_>) -> Result<bool, StoreError> {
        let (filter, update) = claiming(path, file).map_err(|error| failed(file.path, &error))?;
        match self.objects.update_one(filter, update).upsert(true).await {
            Ok(_) => Ok(true),
            Err(error) if duplicate_key(&error) => Ok(false),
            Err(error) => Err(failed(file.path, &error)),
        }
    }

    /// Puts the object of `file` in the bucket unless its record says it is
    /// stored, then marks the record stored.
    async fn upload(&self, file: &Embedded<'_>) -> Result<(), StoreError> {
        let paths = StoredObject::MONGO_FIELDS;
        let stored = paths
            .id()
            .eq(file.hash.clone())
            .and_then(|on_id| Ok(on_id.and(paths.stored().eq(true)?)))
            .map_err(|error| failed(file.path, &error))?;
        let held = self
            .objects
            .find_one(stored.into_document())
            .projection(doc! { "_id": 1_i32 })
            .await
            .map_err(|error| failed(file.path, &error))?;
        if held.is_some() {
            return Ok(());
        }
        match self
            .files
            .write(&file.object, file.bytes, &Precondition::Absent)
            .await
        {
            // A conflict is the same bytes, put there by an earlier save.
            Ok(_) | Err(StoreError::Conflict(_)) => {}
            Err(error) => return Err(error),
        }
        let (on_id, marked) = paths
            .id()
            .eq(file.hash.clone())
            .and_then(|on_id| Ok((on_id, paths.stored().set(true)?)))
            .map_err(|error| failed(file.path, &error))?;
        self.objects
            .update_one(on_id.into_document(), marked.into_document())
            .await
            .map_err(|error| failed(file.path, &error))?;
        Ok(())
    }

    /// Writes `document` at `path` if `precondition` holds, and gives the new
    /// revision: the SHA-256 of the document in archie's form.
    ///
    /// The record keeps each file of the record it replaces that `document`
    /// still embeds, each other claim of the document is taken back, and each
    /// object that leaves unembedded is deleted. Nothing is claimed or uploaded.
    ///
    /// # Errors
    ///
    /// `StoreError::Conflict` when `precondition` does not hold, or
    /// `StoreError::Backend` naming the path when the record nests too deep or
    /// the driver or the bucket fails, naming a kept note that is not UTF-8, or
    /// naming an object whose deletion fails.
    pub async fn write_document(
        &self,
        path: &StorePath,
        document: &RawDocument,
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        let kept = self.still_embedded(path, document).await?;
        let revision = self.save(path, document, &kept, precondition).await?;
        for hash in self.release(path, &kept).await? {
            self.delete_object(&hash).await?;
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
    /// `Conflict`, and none is `NotFound`. Once the record is deleted, each
    /// claim of the document is taken back, and each object that leaves
    /// unembedded is deleted.
    async fn delete(
        &self,
        path: &StorePath,
        precondition: &Precondition,
    ) -> Result<(), StoreError> {
        self.remove_document(path, precondition).await?;
        for hash in self.release(path, &BTreeMap::new()).await? {
            self.delete_object(&hash).await?;
        }
        Ok(())
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

    /// A path no document is at reads as an embedded file: the object that
    /// the record of a document claiming the path names.
    async fn read(&self, path: &StorePath) -> Result<Stored, StoreError> {
        let (document, revision) = match self.read_document(path).await {
            Ok(read) => read,
            Err(StoreError::NotFound(_)) => {
                let bytes = self.read_file(path).await?;
                return Ok(Stored {
                    revision: file::revision(&bytes),
                    bytes,
                });
            }
            Err(error) => return Err(error),
        };
        let text = writer::write(&document, &WriteOptions::default())
            .map_err(|error| failed(path, &error))?;
        Ok(Stored {
            bytes: text.into_bytes(),
            revision,
        })
    }

    /// The object that the record of `document` names for `file` in its
    /// `files`: `NotFound` when no record is at `document`, its `files` do not
    /// hold `file`, or the bucket does not hold the object.
    async fn read_embedded(
        &self,
        document: &StorePath,
        file: &StorePath,
    ) -> Result<Vec<u8>, StoreError> {
        let missing = || StoreError::NotFound(file.clone());
        let name = name_in(document, file.as_str()).ok_or_else(missing)?;
        let held = self.held_files(document).await?;
        let hash = held.get(name).ok_or_else(missing)?;
        match self.files.read(&object_path(hash)?).await {
            Ok(object) => Ok(object.bytes),
            Err(StoreError::NotFound(_)) => Err(missing()),
            Err(error) => Err(error),
        }
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

    /// Claims the object of each file of `files` for the document, uploads
    /// each one its record does not say is stored, saves the document with the
    /// SHA-256 of each file, takes back each claim of the document that it no
    /// longer makes, then deletes each object that leaves unembedded.
    async fn write_with_embedded(
        &self,
        path: &StorePath,
        text: &str,
        files: &BTreeMap<StorePath, Vec<u8>>,
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        let document = parse(text).document;
        let embedded = embedded(path, &document, files)?;
        for file in &embedded {
            self.claim(path, file).await?;
        }
        for file in &embedded {
            self.upload(file).await?;
        }
        let hashes = embedded
            .iter()
            .map(|file| (file.name.to_owned(), file.hash.clone()))
            .collect();
        let revision = self.save(path, &document, &hashes, precondition).await?;
        for hash in self.release(path, &hashes).await? {
            self.delete_object(&hash).await?;
        }
        Ok(revision)
    }
}

fn backend(error: &dyn fmt::Display) -> StoreError {
    StoreError::Backend(error.to_string())
}

/// The filter and the update of a claim of the object of `file` for the
/// document at `path`: the claim added once, and the object's metadata set
/// where the update makes the record.
fn claiming(path: &StorePath, file: &Embedded<'_>) -> Result<(Document, Document), WriteError> {
    let paths = StoredObject::MONGO_FIELDS;
    let claim = Claim {
        document: path.as_str().to_owned(),
        path: file.path.as_str().to_owned(),
    };
    let filter = paths
        .id()
        .eq(file.hash.clone())?
        .and(paths.deleting().ne(true)?);
    let added = bson::serialize_to_bson(&claim)?;
    let update = Update::<StoredObject>::raw(doc! { "$addToSet": { "embedded_by": added } })
        .and(
            paths
                .object()
                .set_on_insert(file.object.as_str().to_owned())?,
        )
        .and(
            paths
                .content_type()
                .set_on_insert(media_type(file.name).to_owned())?,
        )
        .and(paths.size().set_on_insert(file.bytes.len() as u64)?)
        .and(
            paths
                .description()
                .set_on_insert(file.description.clone())?,
        )
        .and(paths.stored().set_on_insert(false)?);
    Ok((filter.into_document(), update.into_document()))
}

fn command_code(error: &Error) -> Option<i32> {
    if let ErrorKind::Command(command) = error.kind.as_ref() {
        Some(command.code)
    } else {
        None
    }
}

/// The alt text of the first embed that names each file of `files`, by its
/// path in the bundle: the embeds of `document` first, then each embedded
/// note's own, from the note's folder.
fn descriptions(
    document: &RawDocument,
    files: &BTreeMap<&str, (&StorePath, &[u8])>,
) -> Result<BTreeMap<String, String>, StoreError> {
    let mut described = BTreeMap::new();
    let mut pending = vec![(String::new(), document.clone())];
    while let Some((folder, mut text)) = pending.pop() {
        let mut alts = Vec::new();
        images_mut(&mut text, &mut |image| {
            alts.push((image.target.clone(), image.alt.clone()));
        })
        .map_err(|error| backend(&error))?;
        for (target, name) in embedded_paths(&text, &folder).map_err(|error| backend(&error))? {
            if described.contains_key(&name) {
                continue;
            }
            if is_note(&name)
                && let Some((file, bytes)) = files.get(name.as_str())
            {
                let note = from_utf8(bytes).map_err(|error| failed(file, &error))?;
                pending.push((parent_of(&name).to_owned(), parse(note).document));
            }
            let alt = alts
                .iter()
                .find(|(written, _)| *written == target)
                .map_or_else(String::new, |(_, alt)| alt.clone());
            described.insert(name, alt);
        }
    }
    Ok(described)
}

fn duplicate_key(error: &Error) -> bool {
    matches!(
        error.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(failure)) if failure.code == DUPLICATE_KEY
    )
}

/// Each file of `files` that a save of `document` at `path` writes.
fn embedded<'save>(
    path: &StorePath,
    document: &RawDocument,
    files: &'save BTreeMap<StorePath, Vec<u8>>,
) -> Result<Vec<Embedded<'save>>, StoreError> {
    let mut named = BTreeMap::new();
    for (file, bytes) in files {
        let name = name_in(path, file.as_str())
            .ok_or_else(|| StoreError::Backend(format!("{file}: not in the folder of {path}")))?;
        named.insert(name, (file, bytes.as_slice()));
    }
    let mut described = descriptions(document, &named)?;
    named
        .into_iter()
        .map(|(name, (file, bytes))| {
            let hash = file::sha256(bytes);
            Ok(Embedded {
                bytes,
                description: described.remove(name).unwrap_or_default(),
                object: object_path(&hash)?,
                hash,
                name,
                path: file,
            })
        })
        .collect()
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

/// Whether an index of `collection` has `key` as its one key, and is unique
/// where `unique` asks it. A collection that does not exist has none.
async fn has_index(
    collection: &Collection<Document>,
    key: &str,
    unique: bool,
) -> Result<bool, StoreError> {
    let mut indexes = match collection.list_indexes().await {
        Ok(indexes) => indexes,
        Err(error) if command_code(&error) == Some(NAMESPACE_NOT_FOUND) => return Ok(false),
        Err(error) => return Err(backend(&error)),
    };
    while indexes.advance().await.map_err(|error| backend(&error))? {
        let index = indexes
            .deserialize_current()
            .map_err(|error| backend(&error))?;
        let unique_index = index.options.and_then(|options| options.unique) == Some(true);
        if (unique_index || !unique) && index.keys.len() == 1 && index.keys.contains_key(key) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The filter and the update that mark the record of the object whose bytes
/// have the SHA-256 `hash` as held by a deletion, where nothing embeds it and
/// no deletion holds it yet.
fn marking(hash: &str) -> Result<(Document, Document), WriteError> {
    let paths = StoredObject::MONGO_FIELDS;
    // Without `deleting`, a second deletion could remove what a later save uploads.
    let filter = paths
        .id()
        .eq(hash.to_owned())?
        .and(paths.embedded_by().size(0))
        .and(paths.deleting().ne(true)?);
    Ok((
        filter.into_document(),
        paths.deleting().set(true)?.into_document(),
    ))
}

/// The path of `file` from the folder of the document at `document`, as a
/// bundle names it, or `None` when `file` is not in that folder.
fn name_in<'file>(document: &StorePath, file: &'file str) -> Option<&'file str> {
    document
        .as_str()
        .rsplit_once('/')
        .map_or(Some(file), |(folder, _)| {
            file.strip_prefix(folder)?.strip_prefix('/')
        })
}

/// The path, in the bucket of the store's files, of the object whose bytes
/// have the SHA-256 `hash`.
fn object_path(hash: &str) -> Result<StorePath, StoreError> {
    StorePath::new(&format!("files/{hash}"))
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

/// The filter and the update that take `claim` out of the record of the
/// object whose bytes have the SHA-256 `hash`.
fn releasing(hash: &str, claim: &Claim) -> Result<(Document, Document), WriteError> {
    let (paths, claims) = (StoredObject::MONGO_FIELDS, Claim::MONGO_FIELDS);
    let held = claims
        .document()
        .eq(claim.document.clone())?
        .and(claims.path().eq(claim.path.clone())?);
    let filter = paths.id().eq(hash.to_owned())?;
    Ok((
        filter.into_document(),
        paths.embedded_by().pull(held).into_document(),
    ))
}

#[cfg(test)]
#[cfg(store_tests)]
mod tests;
