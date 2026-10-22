//! The commands, run over a system and the three standard streams.

use core::fmt::Display;
#[cfg(not(feature = "mongodb"))]
use core::future::{self, Ready};
#[cfg(any(feature = "minio", feature = "mongodb"))]
use std::env::VarError;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use archie_core::bundle::Missing;
use archie_core::forms::{Form, encode};
use archie_core::writer::WriteOptions;
use archie_store::any::AnyStore;
use archie_store::document::{self, DocumentError};
use archie_store::file::FileStore;
#[cfg(feature = "mongodb")]
use archie_store::mongo::MongoStore;
#[cfg(any(feature = "minio", feature = "mongodb"))]
use archie_store::object::{ObjectConfig, ObjectStore};
use archie_store::{Precondition, Revision, Store as _, StoreError, StorePath};
use os_shim::System;
#[cfg(any(feature = "minio", feature = "mongodb"))]
use os_shim::bucket::MinioBucket;

#[cfg(feature = "mongodb")]
use crate::cli::bucket_and_prefix;
use crate::cli::{Cli, Command, FormatArg, StoreArg, SyntaxArg};

/// Why a command stopped: the line after `archie: `, and the exit code.
struct Failure {
    code: u8,
    message: String,
}

/// What `--format`, `--frontmatter` and `--allow-missing` ask for: the form,
/// the options the document is written with, and what an embed with no file
/// does.
struct Settings {
    form: Form,
    missing: Missing,
    options: WriteOptions,
}

impl Failure {
    #[cfg(any(feature = "minio", feature = "mongodb"))]
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
    #[cfg(any(feature = "minio", feature = "mongodb"))]
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

    #[cfg(not(all(feature = "minio", feature = "mongodb")))]
    fn unbuilt(arg: &StoreArg, store: &str) -> Self {
        Self {
            code: 3,
            message: format!("{arg}: this archie is built without the {store} store"),
        }
    }
}

impl Settings {
    fn new(format: FormatArg, frontmatter: SyntaxArg, allow_missing: bool) -> Self {
        let mut options = WriteOptions::default();
        options.frontmatter = frontmatter.into();
        let missing = if allow_missing {
            Missing::Keep
        } else {
            Missing::Refuse
        };
        Self {
            form: format.into(),
            missing,
            options,
        }
    }
}

/// `path` as the command line wrote it, as a path of a store `--store` names.
#[cfg(any(feature = "minio", feature = "mongodb"))]
fn as_written(path: &Path) -> Result<StorePath, Failure> {
    let text = path.to_str().ok_or_else(|| Failure::not_utf8(path))?;
    StorePath::new(text).map_err(|error| Failure::of_store(path, None, &error))
}

/// Writes the document at `path` to `stdout`, or to the file `output`, whole.
/// Markdown to `output` is that file and each file it embeds beside it, unless
/// `embed` asks for the one file.
async fn get(
    system: &dyn System,
    named: Option<&StoreArg>,
    path: &Path,
    settings: &Settings,
    output: Option<&Path>,
    embed: bool,
    stdout: &mut (dyn Write + Send),
) -> Result<(), Failure> {
    if embed && settings.form != Form::Markdown {
        return Err(Failure {
            code: 2,
            message: "--embed writes markdown; it needs -f markdown".to_owned(),
        });
    }
    let (store, at) = open(system, named, path).await?;
    let target = output
        .map(|file| store_path(system, file).map(|place| (file, place)))
        .transpose()?;
    let bundle = document::get(&store, &at, settings.missing)
        .await
        .map_err(|error| Failure::of_document(path, &at, &error))?;
    if let Some((file, place)) = &target
        && settings.form == Form::Markdown
        && !embed
    {
        let _: Revision = document::put_bundle(
            &machine(system),
            place,
            &bundle,
            &settings.options,
            &Precondition::None,
        )
        .await
        .map_err(|error| Failure::of_document(file, place, &error))?;
        return Ok(());
    }
    let bytes = encode(&bundle, settings.form, &settings.options)
        .map_err(|error| Failure::naming(path, None, 1, &error))?;
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

/// The `MinIO` store `arg` names, `bucket` under `prefix` on the server the
/// environment names, and `path` as a path of it.
#[cfg(feature = "minio")]
fn minio<'system>(
    system: &dyn System,
    arg: &StoreArg,
    bucket: &str,
    prefix: Option<&str>,
    path: &Path,
) -> Result<(AnyStore<'system>, StorePath), Failure> {
    let under = prefix
        .map(StorePath::new)
        .transpose()
        .map_err(|error| Failure::store(arg, &error))?;
    let store = objects(system, bucket, under)?;
    let at = as_written(path)?;
    Ok((AnyStore::Minio(store), at))
}

/// The bucket a MongoDB store keeps its files in: `ARCHIE_MONGODB_BUCKET`,
/// `<bucket>` or `<bucket>/<prefix>`, on the `MinIO` server the environment
/// names.
#[cfg(feature = "mongodb")]
fn mongodb_files(system: &dyn System) -> Result<ObjectStore<MinioBucket>, Failure> {
    let (name, holds) = (
        "ARCHIE_MONGODB_BUCKET",
        "the MinIO bucket of a MongoDB store's files",
    );
    let place = variable(system, name, holds)?;
    let (bucket, prefix) = bucket_and_prefix(&place).ok_or_else(|| {
        Failure::configuration(format!(
            "{name} is not <bucket> or <bucket>/<prefix>: it holds {holds}"
        ))
    })?;
    let under = prefix
        .map(StorePath::new)
        .transpose()
        .map_err(|error| Failure::configuration(format!("{name}: {error}")))?;
    objects(system, bucket, under)
}

/// The bucket `bucket`, its keys under `prefix`, on the `MinIO` server the
/// environment names.
#[cfg(any(feature = "minio", feature = "mongodb"))]
fn objects(
    system: &dyn System,
    bucket: &str,
    prefix: Option<StorePath>,
) -> Result<ObjectStore<MinioBucket>, Failure> {
    let endpoint = variable(
        system,
        "ARCHIE_MINIO_ENDPOINT",
        "the MinIO server's address",
    )?;
    let region = variable(
        system,
        "ARCHIE_MINIO_REGION",
        "the region the MinIO server expects",
    )?;
    let config = ObjectConfig::new(endpoint, region, bucket.to_owned(), prefix);
    // `open` fails only on a key missing from the environment.
    ObjectStore::open(system, config).map_err(|error| Failure::configuration(error.to_string()))
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
        #[cfg(feature = "minio")]
        StoreArg::Minio { bucket, prefix } => minio(system, arg, bucket, prefix.as_deref(), path),
        #[cfg(not(feature = "minio"))]
        StoreArg::Minio {
            bucket: _,
            prefix: _,
        } => Err(Failure::unbuilt(arg, "MinIO")),
        StoreArg::Mongodb {
            collection,
            database,
        } => {
            let uri = variable(
                system,
                "ARCHIE_MONGODB_URI",
                "the MongoDB connection string",
            )?;
            let files = mongodb_files(system)?;
            let at = as_written(path)?;
            let store = MongoStore::open(&uri, database, collection, files)
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
    let opened = match arg {
        #[cfg(feature = "minio")]
        StoreArg::Minio { bucket, prefix } => minio(system, arg, bucket, prefix.as_deref(), path),
        #[cfg(not(feature = "minio"))]
        StoreArg::Minio {
            bucket: _,
            prefix: _,
        } => Err(Failure::unbuilt(arg, "MinIO")),
        StoreArg::Mongodb {
            collection: _,
            database: _,
        } => Err(Failure::unbuilt(arg, "MongoDB")),
    };
    future::ready(opened)
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
            allow_missing,
            embed,
            format,
            frontmatter,
            output,
            store,
        } => {
            let settings = Settings::new(*format, *frontmatter, *allow_missing);
            get(
                system,
                store.as_ref(),
                path,
                &settings,
                output.as_deref(),
                *embed,
                stdout,
            )
            .await
        }
        Command::Write {
            path,
            allow_missing,
            format,
            frontmatter,
            input,
            store,
        } => {
            let settings = Settings::new(*format, *frontmatter, *allow_missing);
            write(
                system,
                store.as_ref(),
                path,
                &settings,
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

/// The value of the variable `name`, which holds `holds`; the error names the
/// variable, never its value.
#[cfg(any(feature = "minio", feature = "mongodb"))]
fn variable(system: &dyn System, name: &str, holds: &str) -> Result<String, Failure> {
    system.env_var(name).map_err(|error| {
        let what = match error {
            VarError::NotPresent => "is not set",
            VarError::NotUnicode(_) => "is not UTF-8",
        };
        Failure::configuration(format!("{name} {what}: it holds {holds}"))
    })
}

/// Stores at `path` the document read from `stdin`, or from the file `input`,
/// replacing any file there. Markdown from `input` brings each file it embeds
/// from beside it.
async fn write(
    system: &dyn System,
    named: Option<&StoreArg>,
    path: &Path,
    settings: &Settings,
    input: Option<&Path>,
    stdin: &mut (dyn Read + Send),
) -> Result<(), Failure> {
    let (store, at) = open(system, named, path).await?;
    let stored = if let Some(file) = input
        && settings.form == Form::Markdown
    {
        let source = store_path(system, file)?;
        let bundle = document::get(&machine(system), &source, settings.missing)
            .await
            .map_err(|error| Failure::of_document(file, &source, &error))?;
        document::put_bundle(&store, &at, &bundle, &settings.options, &Precondition::None).await
    } else {
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
        document::put(
            &store,
            &at,
            settings.form,
            &bytes,
            &settings.options,
            &Precondition::None,
            settings.missing,
        )
        .await
    };
    let _: Revision = stored.map_err(|error| Failure::of_document(path, &at, &error))?;
    Ok(())
}

#[cfg(test)]
mod tests;
