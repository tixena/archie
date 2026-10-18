//! Files kept as objects in one bucket, through os-shim's `Bucket`.

use std::env::VarError;

use os_shim::System;
use os_shim::bucket::{Bucket, Condition, MinioBucket, MinioConfig, ObjectError, ObjectErrorKind};

use crate::{Capabilities, Precondition, Revision, Store, StoreError, StorePath, Stored};

/// The longest key S3 takes, in bytes.
const LONGEST_KEY: usize = 1024;

/// Markdown files in one bucket, under an optional key prefix. Over
/// `MinioBucket`, a delete's precondition is checked by a request of its own
/// before the delete, not in one step with it.
#[derive(Debug)]
pub struct ObjectStore<B> {
    bucket: B,
    prefix: Option<StorePath>,
}

impl<B: Bucket> ObjectStore<B> {
    /// The key of the object that holds the file at `path`.
    fn key(&self, path: &StorePath) -> String {
        self.prefix.as_ref().map_or_else(
            || path.as_str().to_owned(),
            |prefix| format!("{prefix}/{path}"),
        )
    }

    /// A store over `bucket`, its keys under `prefix`.
    #[must_use]
    pub const fn new(bucket: B, prefix: Option<StorePath>) -> Self {
        Self { bucket, prefix }
    }
}

impl ObjectStore<MinioBucket> {
    /// A store over the `MinIO` server `config` names, its keys read from the
    /// environment.
    ///
    /// # Errors
    ///
    /// `Backend` naming the variable when `AWS_ACCESS_KEY_ID` or
    /// `AWS_SECRET_ACCESS_KEY` is not set.
    pub fn open(system: &dyn System, config: ObjectConfig) -> Result<Self, StoreError> {
        let access_key = variable(system, "AWS_ACCESS_KEY_ID")?;
        let secret_key = variable(system, "AWS_SECRET_ACCESS_KEY")?;
        let minio = MinioConfig::new(
            config.endpoint,
            config.region,
            config.bucket,
            access_key,
            secret_key,
        );
        Ok(Self::new(MinioBucket::new(minio), config.prefix))
    }
}

impl<B: Bucket> Store for ObjectStore<B> {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_preconditions: true,
        }
    }

    /// Under `Precondition::None` or `Precondition::Absent`, a delete where no
    /// file is succeeds, as S3 answers it.
    async fn delete(
        &self,
        path: &StorePath,
        precondition: &Precondition,
    ) -> Result<(), StoreError> {
        self.bucket
            .delete(&self.key(path), &condition(precondition))
            .await
            .map_err(|error| failure(path, &error))
    }

    async fn list(&self, prefix: Option<&StorePath>) -> Result<Vec<StorePath>, StoreError> {
        let root = self
            .prefix
            .as_ref()
            .map_or_else(String::new, |store| format!("{store}/"));
        let listed = prefix.map_or_else(|| root.clone(), |folder| format!("{root}{folder}/"));
        let keys = self.bucket.list(&listed).await.map_err(|error| {
            StoreError::Backend(
                prefix.map_or_else(|| error.to_string(), |folder| format!("{folder}: {error}")),
            )
        })?;
        let mut paths = keys
            .iter()
            .filter(|key| !key.ends_with('/'))
            .map(|key| {
                key.strip_prefix(&root)
                    .map_or_else(|| Err(StoreError::InvalidPath(key.clone())), StorePath::new)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort_unstable();
        Ok(paths)
    }

    async fn read(&self, path: &StorePath) -> Result<Stored, StoreError> {
        let object = self
            .bucket
            .get(&self.key(path))
            .await
            .map_err(|error| failure(path, &error))?;
        Ok(Stored {
            bytes: object.bytes,
            revision: Revision(object.etag),
        })
    }

    async fn write(
        &self,
        path: &StorePath,
        bytes: &[u8],
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        let key = self.key(path);
        if key.len() > LONGEST_KEY {
            return Err(StoreError::InvalidPath(path.as_str().to_owned()));
        }
        let etag = self
            .bucket
            .put(&key, bytes, &condition(precondition))
            .await
            .map_err(|error| {
                // A put with `If-Match` where no object is answers 404 on RustFS.
                let unmatched = matches!(precondition, Precondition::Matches(_))
                    && error.kind() == ObjectErrorKind::NotFound;
                if unmatched {
                    StoreError::Conflict(path.clone())
                } else {
                    failure(path, &error)
                }
            })?;
        Ok(Revision(etag))
    }
}

/// Where the bucket is: the `MinIO` server, its region, the bucket, and the
/// key prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ObjectConfig {
    pub bucket: String,
    /// The server's address, such as `http://127.0.0.1:9000`; required, no
    /// default.
    pub endpoint: String,
    pub prefix: Option<StorePath>,
    pub region: String,
}

impl ObjectConfig {
    #[must_use]
    pub const fn new(
        endpoint: String,
        region: String,
        bucket: String,
        prefix: Option<StorePath>,
    ) -> Self {
        Self {
            bucket,
            endpoint,
            prefix,
            region,
        }
    }
}

/// The bucket's condition for `precondition`, a revision being the entity tag.
fn condition(precondition: &Precondition) -> Condition {
    match precondition {
        Precondition::Absent => Condition::Absent,
        Precondition::Matches(revision) => Condition::ETag(revision.0.clone()),
        Precondition::None => Condition::None,
    }
}

/// `NotFound` or `Conflict` of `path` when `error` says so, else `Backend`
/// naming `path`.
fn failure(path: &StorePath, error: &ObjectError) -> StoreError {
    match error.kind() {
        ObjectErrorKind::NotFound => StoreError::NotFound(path.clone()),
        ObjectErrorKind::PreconditionFailed => StoreError::Conflict(path.clone()),
        ObjectErrorKind::Other | _ => StoreError::Backend(format!("{path}: {error}")),
    }
}

/// The value of the variable `name`; the error names the variable, never
/// its value.
fn variable(system: &dyn System, name: &str) -> Result<String, StoreError> {
    system.env_var(name).map_err(|error| match error {
        VarError::NotPresent => StoreError::Backend(format!("`{name}` is not set")),
        VarError::NotUnicode(_) => StoreError::Backend(format!("`{name}` is not Unicode")),
    })
}

#[cfg(test)]
mod tests;
