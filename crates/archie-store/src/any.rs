//! One type for whichever store a caller opened.

use crate::file::FileStore;
#[cfg(feature = "mongodb")]
use crate::mongo::MongoStore;
use crate::{Capabilities, Precondition, Revision, Store, StoreError, StorePath, Stored};

/// Whichever store a caller opened. Each variant is behind its store's
/// cargo feature.
#[non_exhaustive]
pub enum AnyStore<'system> {
    File(FileStore<'system>),
    #[cfg(feature = "mongodb")]
    Mongo(MongoStore),
}

impl Store for AnyStore<'_> {
    fn capabilities(&self) -> Capabilities {
        match self {
            Self::File(store) => store.capabilities(),
            #[cfg(feature = "mongodb")]
            Self::Mongo(store) => store.capabilities(),
        }
    }

    async fn delete(
        &self,
        path: &StorePath,
        precondition: &Precondition,
    ) -> Result<(), StoreError> {
        match self {
            Self::File(store) => store.delete(path, precondition).await,
            #[cfg(feature = "mongodb")]
            Self::Mongo(store) => store.delete(path, precondition).await,
        }
    }

    async fn list(&self, prefix: Option<&StorePath>) -> Result<Vec<StorePath>, StoreError> {
        match self {
            Self::File(store) => store.list(prefix).await,
            #[cfg(feature = "mongodb")]
            Self::Mongo(store) => store.list(prefix).await,
        }
    }

    async fn read(&self, path: &StorePath) -> Result<Stored, StoreError> {
        match self {
            Self::File(store) => store.read(path).await,
            #[cfg(feature = "mongodb")]
            Self::Mongo(store) => store.read(path).await,
        }
    }

    async fn write(
        &self,
        path: &StorePath,
        bytes: &[u8],
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        match self {
            Self::File(store) => store.write(path, bytes, precondition).await,
            #[cfg(feature = "mongodb")]
            Self::Mongo(store) => store.write(path, bytes, precondition).await,
        }
    }
}

#[cfg(test)]
mod tests;
