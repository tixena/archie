//! The commands, run over a system and the three standard streams.

use core::fmt::Display;
#[cfg(not(feature = "mongodb"))]
use core::future::{self, Ready};
#[cfg(feature = "mongodb")]
use std::env::VarError;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use archie_core::forms::Form;
use archie_core::writer::WriteOptions;
use archie_store::any::AnyStore;
use archie_store::document::{self, DocumentError};
use archie_store::file::FileStore;
#[cfg(feature = "mongodb")]
use archie_store::mongo::MongoStore;
use archie_store::{Precondition, Revision, Store as _, StoreError, StorePath};
use os_shim::System;

use crate::cli::{Cli, Command, StoreArg, SyntaxArg};

/// Why a command stopped: the line after `archie: `, and the exit code.
struct Failure {
    code: u8,
    message: String,
}

impl Failure {
    #[cfg(feature = "mongodb")]
    const fn configuration(message: String) -> Self {
        Self { code: 3, message }
    }

    /// `error` on one line naming `path` as the command line gave it, in place
    /// of the store path `at` that the error's message opens with.
    fn naming(path: &Path, at: Option<&StorePath>, code: u8, error: &dyn Display) -> Self {
        let text = error.to_string();
        let what = at
            .and_then(|place| text.strip_prefix(place.as_str()))
            .and_then(|rest| rest.strip_prefix(": "))
            .unwrap_or(&text);
        Self {
            code,
            message: format!("{}: {what}", path.display()),
        }
    }

    fn not_utf8(path: &Path) -> Self {
        Self {
            code: 2,
            message: format!("{}: a path that is not UTF-8", path.display()),
        }
    }

    fn of_document(path: &Path, at: &StorePath, error: &DocumentError) -> Self {
        let code = match error {
            DocumentError::Form { error: _, path: _ } => 1,
            DocumentError::Store(store) => store_code(store),
            _ => 4,
        };
        Self::naming(path, Some(at), code, error)
    }

    fn of_store(path: &Path, at: Option<&StorePath>, error: &StoreError) -> Self {
        Self::naming(path, at, store_code(error), error)
    }

    /// The store `arg` names failed to open.
    #[cfg(feature = "mongodb")]
    fn store(arg: &StoreArg, error: &StoreError) -> Self {
        Self {
            code: store_code(error),
            message: format!("{arg}: {error}"),
        }
    }

    fn stream(name: &str, error: &io::Error) -> Self {
        Self {
            code: 4,
            message: format!("{name}: {error}"),
        }
    }

    fn unbuilt(arg: &StoreArg, store: &str) -> Self {
        Self {
            code: 3,
            message: format!("{arg}: this archie is built without the {store} store"),
        }
    }
}

/// `path` as the command line wrote it, as a path of a store `--store` names.
#[cfg(feature = "mongodb")]
fn as_written(path: &Path) -> Result<StorePath, Failure> {
    let text = path.to_str().ok_or_else(|| Failure::not_utf8(path))?;
    StorePath::new(text).map_err(|error| Failure::of_store(path, None, &error))
}

/// Writes the document at `path` in `form` to `stdout`, or to the file
/// `output`, whole.
async fn get(
    system: &dyn System,
    named: Option<&StoreArg>,
    path: &Path,
    form: Form,
    options: &WriteOptions,
    output: Option<&Path>,
    stdout: &mut (dyn Write + Send),
) -> Result<(), Failure> {
    let (store, at) = open(system, named, path).await?;
    let target = output
        .map(|file| store_path(system, file).map(|place| (file, place)))
        .transpose()?;
    let bytes = document::get(&store, &at, form, options)
        .await
        .map_err(|error| Failure::of_document(path, &at, &error))?;
    if let Some((file, place)) = target {
        let _: Revision = machine(system)
            .write(&place, &bytes, &Precondition::None)
            .await
            .map_err(|error| Failure::of_store(file, Some(&place), &error))?;
        return Ok(());
    }
    stdout
        .write_all(&bytes)
        .and_then(|()| stdout.flush())
        .map_err(|error| Failure::stream("standard output", &error))
}

/// The files of the machine, as a store whose root is `/`.
fn machine(system: &dyn System) -> FileStore<'_> {
    FileStore::new(PathBuf::from("/"), system)
}

/// The connection string the environment holds in `ARCHIE_MONGODB_URI`.
#[cfg(feature = "mongodb")]
fn mongodb_uri(system: &dyn System) -> Result<String, Failure> {
    system.env_var("ARCHIE_MONGODB_URI").map_err(|error| {
        let what = match error {
            VarError::NotPresent => "is not set",
            VarError::NotUnicode(_) => "is not UTF-8",
        };
        Failure::configuration(format!(
            "ARCHIE_MONGODB_URI {what}: it holds the MongoDB connection string"
        ))
    })
}

/// The store `named` names, the file system's when it is `None`, and `path`
/// as a path of it.
#[cfg(feature = "mongodb")]
async fn open<'system>(
    system: &'system dyn System,
    named: Option<&StoreArg>,
    path: &Path,
) -> Result<(AnyStore<'system>, StorePath), Failure> {
    let Some(arg) = named else {
        return Ok((AnyStore::File(machine(system)), store_path(system, path)?));
    };
    match arg {
        StoreArg::Minio {
            bucket: _,
            prefix: _,
        } => Err(Failure::unbuilt(arg, "MinIO")),
        StoreArg::Mongodb {
            collection,
            database,
        } => {
            let uri = mongodb_uri(system)?;
            let at = as_written(path)?;
            let store = MongoStore::open(&uri, database, collection)
                .await
                .map_err(|error| Failure::store(arg, &error))?;
            Ok((AnyStore::Mongo(store), at))
        }
    }
}

/// The store `named` names, the file system's when it is `None`, and `path`
/// as a path of it: ready at once, as no store of this build connects.
#[cfg(not(feature = "mongodb"))]
fn open<'system>(
    system: &'system dyn System,
    named: Option<&StoreArg>,
    path: &Path,
) -> Ready<Result<(AnyStore<'system>, StorePath), Failure>> {
    let Some(arg) = named else {
        let at = store_path(system, path);
        return future::ready(at.map(|place| (AnyStore::File(machine(system)), place)));
    };
    let unbuilt = match arg {
        StoreArg::Minio {
            bucket: _,
            prefix: _,
        } => "MinIO",
        StoreArg::Mongodb {
            collection: _,
            database: _,
        } => "MongoDB",
    };
    future::ready(Err(Failure::unbuilt(arg, unbuilt)))
}

fn options(syntax: SyntaxArg) -> WriteOptions {
    let mut options = WriteOptions::default();
    options.frontmatter = syntax.into();
    options
}

/// Runs `cli` over `system` and the three streams, and gives the exit code.
pub async fn run(
    cli: &Cli,
    system: &dyn System,
    stdin: &mut (dyn Read + Send),
    stdout: &mut (dyn Write + Send),
    stderr: &mut (dyn Write + Send),
) -> ExitCode {
    let done = match &cli.command {
        Command::Get {
            path,
            format,
            frontmatter,
            output,
            store,
        } => {
            let form = (*format).into();
            let options = options(*frontmatter);
            get(
                system,
                store.as_ref(),
                path,
                form,
                &options,
                output.as_deref(),
                stdout,
            )
            .await
        }
        Command::Write {
            path,
            format,
            frontmatter,
            input,
            store,
        } => {
            let form = (*format).into();
            let options = options(*frontmatter);
            write(
                system,
                store.as_ref(),
                path,
                form,
                &options,
                input.as_deref(),
                stdin,
            )
            .await
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            let _: io::Result<()> = writeln!(stderr, "archie: {}", failure.message);
            ExitCode::from(failure.code)
        }
    }
}

/// The exit code of a store's error.
const fn store_code(error: &StoreError) -> u8 {
    match error {
        StoreError::InvalidPath(_) => 2,
        StoreError::NotFound(_) => 6,
        StoreError::Backend(_) | StoreError::Conflict(_) | StoreError::Unsupported(_) | _ => 4,
    }
}

/// `path` against the current folder, `.` and `..` resolved from its text
/// without following links, as a path of the store whose root is `/`.
fn store_path(system: &dyn System, path: &Path) -> Result<StorePath, Failure> {
    let full = system
        .current_dir()
        .map_err(|error| Failure::stream("the current folder", &error))?
        .join(path);
    let mut parts = Vec::new();
    for component in full.components() {
        match component {
            Component::Normal(part) => {
                parts.push(part.to_str().ok_or_else(|| Failure::not_utf8(path))?);
            }
            Component::ParentDir => {
                parts.pop();
            }
            Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        }
    }
    StorePath::new(&parts.join("/")).map_err(|error| Failure::of_store(path, None, &error))
}

/// Stores at `path` the document in `form` read from `stdin`, or from the
/// file `input`, replacing any file there.
async fn write(
    system: &dyn System,
    named: Option<&StoreArg>,
    path: &Path,
    form: Form,
    options: &WriteOptions,
    input: Option<&Path>,
    stdin: &mut (dyn Read + Send),
) -> Result<(), Failure> {
    let (store, at) = open(system, named, path).await?;
    let bytes = if let Some(file) = input {
        let source = store_path(system, file)?;
        machine(system)
            .read(&source)
            .await
            .map_err(|error| Failure::of_store(file, Some(&source), &error))?
            .bytes
    } else {
        let mut bytes = Vec::new();
        stdin
            .read_to_end(&mut bytes)
            .map_err(|error| Failure::stream("standard input", &error))?;
        bytes
    };
    let _: Revision = document::put(&store, &at, form, &bytes, options, &Precondition::None)
        .await
        .map_err(|error| Failure::of_document(path, &at, &error))?;
    Ok(())
}

#[cfg(test)]
mod tests;
