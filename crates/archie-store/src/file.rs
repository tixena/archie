//! The store over a file system, through os-shim.

use core::fmt::{self, Write as _};
use core::future;
use core::sync::atomic::{AtomicU64, Ordering};
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::process;

use os_shim::System;
use sha2::{Digest as _, Sha256};

use crate::{Capabilities, Precondition, Revision, Store, StoreError, StorePath, Stored};

/// The count in a temporary name, shared by every store in the process.
static WRITES: AtomicU64 = AtomicU64::new(0);

/// Files under a root folder, read and written through os-shim. A revision
/// is the SHA-256 of the file's bytes.
pub struct FileStore<'system> {
    follow_links: bool,
    hidden: bool,
    root: PathBuf,
    system: &'system dyn System,
}

impl<'system> FileStore<'system> {
    /// The bytes of the file at `path`, or `None` when no file is there.
    fn bytes(&self, path: &StorePath) -> Result<Option<Vec<u8>>, StoreError> {
        let (folder, name) = self.place(path);
        let mut reader = match self.system.open(&folder.join(name)) {
            Ok(reader) => reader,
            Err(error) if missing(&error) => return Ok(None),
            Err(error) => return Err(backend(path, &error)),
        };
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|error| backend(path, &error))?;
        Ok(Some(bytes))
    }

    fn check(&self, path: &StorePath, precondition: &Precondition) -> Result<(), StoreError> {
        let holds = match precondition {
            Precondition::Absent => self.bytes(path)?.is_none(),
            Precondition::Matches(expected) => self
                .bytes(path)?
                .is_some_and(|bytes| revision(&bytes) == *expected),
            Precondition::None => true,
        };
        if holds {
            Ok(())
        } else {
            Err(StoreError::Conflict(path.clone()))
        }
    }

    fn delete_file(&self, path: &StorePath, precondition: &Precondition) -> Result<(), StoreError> {
        self.check(path, precondition)?;
        let (folder, name) = self.place(path);
        self.system
            .remove_file(&folder.join(name))
            .map_err(|error| failure(path, &error))
    }

    fn list_files(&self, prefix: Option<&StorePath>) -> Result<Vec<StorePath>, StoreError> {
        let mut folder = self.root.clone();
        folder.extend(prefix.map_or("", StorePath::as_str).split_terminator('/'));
        let entries = self
            .system
            .walk_dir(&folder, self.follow_links, self.hidden)
            .map_err(|error| StoreError::Backend(format!("{}: {error}", folder.display())))?;
        let mut paths = entries
            .iter()
            .filter(|entry| entry.is_file)
            .map(|entry| self.store_path(&entry.path))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort_unstable();
        Ok(paths)
    }

    /// The two flags `walk_dir` takes, `false` by default.
    #[must_use]
    pub fn listing(self, follow_links: bool, hidden: bool) -> Self {
        Self {
            follow_links,
            hidden,
            ..self
        }
    }

    /// A store whose listing follows no link and leaves hidden entries out.
    #[must_use]
    pub fn new(root: PathBuf, system: &'system dyn System) -> Self {
        Self {
            follow_links: false,
            hidden: false,
            root,
            system,
        }
    }

    /// The folder that holds the file at `path`, and the file's name.
    fn place<'path>(&self, path: &'path StorePath) -> (PathBuf, &'path str) {
        let (folders, name) = path
            .as_str()
            .rsplit_once('/')
            .unwrap_or(("", path.as_str()));
        let mut folder = self.root.clone();
        folder.extend(folders.split_terminator('/'));
        (folder, name)
    }

    fn read_file(&self, path: &StorePath) -> Result<Stored, StoreError> {
        let bytes = self
            .bytes(path)?
            .ok_or_else(|| StoreError::NotFound(path.clone()))?;
        let revision = revision(&bytes);
        Ok(Stored { bytes, revision })
    }

    /// The store path of `file`, a file `walk_dir` found under the root.
    fn store_path(&self, file: &Path) -> Result<StorePath, StoreError> {
        let relative = file
            .strip_prefix(&self.root)
            .map_err(|error| StoreError::Backend(format!("{}: {error}", file.display())))?;
        let text = relative
            .components()
            .map(|component| component.as_os_str().to_str())
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| StoreError::InvalidPath(relative.to_string_lossy().into_owned()))?
            .join("/");
        StorePath::new(&text)
    }

    /// A name beside the target, unique to this write in this process.
    fn temporary(folder: &Path, name: &str) -> PathBuf {
        let count = WRITES.fetch_add(1, Ordering::Relaxed);
        folder.join(format!(".{name}.archie-write-{}-{count}", process::id()))
    }

    fn write_file(
        &self,
        path: &StorePath,
        bytes: &[u8],
        precondition: &Precondition,
    ) -> Result<Revision, StoreError> {
        self.check(path, precondition)?;
        let (folder, name) = self.place(path);
        self.system
            .create_dir_all(&folder)
            .map_err(|error| backend(path, &error))?;
        let temporary = Self::temporary(&folder, name);
        let written = self
            .system
            .write(&temporary, bytes)
            .and_then(|()| self.system.rename(&temporary, &folder.join(name)));
        if let Err(error) = written {
            let _: io::Result<()> = self.system.remove_file(&temporary);
            return Err(backend(path, &error));
        }
        Ok(revision(bytes))
    }
}

/// os-shim is synchronous, so each operation is done when it is called and its
/// future is ready.
impl Store for FileStore<'_> {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            atomic_preconditions: false,
        }
    }

    fn delete(
        &self,
        path: &StorePath,
        precondition: &Precondition,
    ) -> impl Future<Output = Result<(), StoreError>> + Send {
        future::ready(self.delete_file(path, precondition))
    }

    fn list(
        &self,
        prefix: Option<&StorePath>,
    ) -> impl Future<Output = Result<Vec<StorePath>, StoreError>> + Send {
        future::ready(self.list_files(prefix))
    }

    fn read(&self, path: &StorePath) -> impl Future<Output = Result<Stored, StoreError>> + Send {
        future::ready(self.read_file(path))
    }

    fn write(
        &self,
        path: &StorePath,
        bytes: &[u8],
        precondition: &Precondition,
    ) -> impl Future<Output = Result<Revision, StoreError>> + Send {
        future::ready(self.write_file(path, bytes, precondition))
    }
}

fn backend(path: &StorePath, error: &io::Error) -> StoreError {
    StoreError::Backend(format!("{path}: {error}"))
}

/// `NotFound` for a missing file, `Backend` for any other failure.
fn failure(path: &StorePath, error: &io::Error) -> StoreError {
    if missing(error) {
        StoreError::NotFound(path.clone())
    } else {
        backend(path, error)
    }
}

/// Whether `error` says no file is at the path: nothing there, or a file
/// where a folder of the path should be.
fn missing(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

/// `sha256:` and the 64 lowercase hexadecimal digits of the hash of `bytes`.
fn revision(bytes: &[u8]) -> Revision {
    let mut text = String::from("sha256:");
    for byte in Sha256::digest(bytes) {
        let _: fmt::Result = write!(text, "{byte:02x}");
    }
    Revision(text)
}

#[cfg(test)]
mod tests;
