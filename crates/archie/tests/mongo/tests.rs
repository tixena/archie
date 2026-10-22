use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command as Process, Stdio};

use archie_core::bundle::Missing;
use archie_core::forms::{Form, encode};
use archie_core::writer::WriteOptions;
use archie_store::any::AnyStore;
use archie_store::document::{get, put};
use archie_store::mongo::MongoStore;
use archie_store::{Precondition, StorePath};
use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use mongodb::Client;
use predicates::ord::eq;
use tokio::runtime::{Builder, Runtime};

const DATABASE: &str = "archie_test_binary_copies";

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../archie-core/src/parser/tests/fixtures"
);

/// The 21 markdown fixture files of archie-core, by name.
fn fixtures() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(FIXTURES)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .map(|path| {
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            (name, fs::read_to_string(&path).unwrap())
        })
        .collect();
    files.sort_unstable();
    assert_eq!(files.len(), 21);
    files
}

/// archie's form of `source` as the MongoDB store keeps it, an offset written
/// as UTC: put at `name` in `expected` and got back, through the library.
fn kept_by_mongodb(
    runtime: &Runtime,
    expected: &AnyStore<'_>,
    name: &str,
    source: &str,
) -> Vec<u8> {
    let path = StorePath::new(name).unwrap();
    let options = WriteOptions::default();
    runtime.block_on(async {
        put(
            expected,
            &path,
            Form::Markdown,
            source.as_bytes(),
            &options,
            &Precondition::None,
            Missing::Keep,
        )
        .await
        .unwrap();
        let bundle = get(expected, &path, Missing::Keep).await.unwrap();
        encode(&bundle, Form::Markdown, &options).unwrap()
    })
}

/// Runs `archie get name` in `root` piped into `archie write name --store
/// store`, both with `--allow-missing`, and asserts both runs succeeded.
fn pipe(root: &Path, name: &str, store: &str) {
    let mut get = Process::new(cargo_bin("archie"))
        .current_dir(root)
        .args(["get", name, "--allow-missing"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let written = Process::new(cargo_bin("archie"))
        .current_dir(root)
        .args(["write", name, "--store", store, "--allow-missing"])
        .env("ARCHIE_MONGODB_URI", uri())
        .stdin(get.stdout.take().unwrap())
        .status()
        .unwrap();
    assert!(get.wait().unwrap().success(), "{name}");
    assert!(written.success(), "{name}");
}

/// The connection string `just test-stores` sets.
fn uri() -> String {
    env::var("ARCHIE_TEST_MONGODB_URI").unwrap()
}

#[test]
fn each_fixture_piped_into_mongodb_and_read_back_is_archies_form_as_mongodb_keeps_it() {
    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
    let opened = runtime.block_on(MongoStore::open(&uri(), DATABASE, "expected"));
    let expected = AnyStore::Mongo(opened.unwrap());
    let folder = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let root = folder.path();
    let store = format!("mongodb:{DATABASE}/files");
    // `obsidian_note.md` embeds a picture this folder does not hold.
    for (name, source) in fixtures() {
        fs::write(root.join(&name), &source).unwrap();
        pipe(root, &name, &store);
        let kept = kept_by_mongodb(&runtime, &expected, &name, &source);
        Command::cargo_bin("archie")
            .unwrap()
            .current_dir(root)
            .args(["get", &name, "--store", &store, "--allow-missing"])
            .env("ARCHIE_MONGODB_URI", uri())
            .assert()
            .success()
            .stdout(eq(kept.as_slice()));
    }
    runtime.block_on(async {
        let client = Client::with_uri_str(uri()).await.unwrap();
        client.database(DATABASE).drop().await.unwrap();
    });
}
