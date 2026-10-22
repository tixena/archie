//! The store against the MongoDB and `RustFS` servers `just test-stores`
//! starts, each test in a database of its own, dropped when the test ends, and
//! its files under a key prefix of its own.

use alloc::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use archie_core::bundle::{BundledDocument, Missing};
use archie_core::forms::{Form, encode};
use archie_core::parser::ast::{Mapping, RawDocument, Value};
use archie_core::parser::{from_json, parse, to_json};
use archie_core::writer::{WriteOptions, write};
use bson::oid::ObjectId;
use bson::{DateTime, Document, doc};
use mongodb::options::IndexOptions;
use mongodb::{Client, Collection, IndexModel};
use os_shim::bucket::MinioBucket;
use os_shim::mock::MemorySystem;
use os_shim::real::RealSystem;

use super::{MongoStore, embedded};
use crate::any::AnyStore;
use crate::document::{get, put, put_bundle};
use crate::file::{self, FileStore};
use crate::mongo::record::{
    Claim, StoredDocument, StoredObject, from_record, frontmatter_is, revision_of, to_record,
};
use crate::object::{ObjectConfig, ObjectStore};
use crate::{Precondition, Store as _, StoreError, StorePath, Stored};

const FIXTURES: [&str; 21] = [
    include_str!("../../../archie-core/src/parser/tests/fixtures/api_reference.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/book_page.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/changelog.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/contact.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/daily_note.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/docusaurus_page.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/essay.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/hugo_post.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/hugo_post_with_table.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/issue_template.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/jekyll_post.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/meeting_notes.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/obsidian_note.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/pandoc_paper.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/readme.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/recipe.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/slash_command.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/task.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/toml_multi_line_string.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/yaml_list_of_mappings.md"),
    include_str!("../../../archie-core/src/parser/tests/fixtures/zola_page.md"),
];

const FORMS: [Form; 3] = [Form::Json, Form::Markdown, Form::Binary];

const OTHER_PNG: &[u8] = b"\x89PNG\r\n\x1a\n another picture";

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n not a real picture";

const REPORT: &str = "# Report\n\n![Logo](media/logo.png)\n";

/// Archie's form of `source` once its record has been through `to_record`
/// and `from_record`, as MongoDB gives it back.
fn back_from_mongodb(source: &str) -> String {
    let options = WriteOptions::default();
    let archie = write(&parse(source).document, &options).unwrap();
    let stored = StoredDocument {
        document: parse(&archie).document,
        files: BTreeMap::new(),
        id: ObjectId::new(),
        path: "a.md".to_owned(),
        revision: revision_of(&archie).0,
    };
    let back = from_record(to_record(&stored).unwrap()).unwrap();
    write(&back.document, &options).unwrap()
}

/// A claim of the document at `document` on the file at `path`.
fn claim(document: &str, path: &str) -> Claim {
    Claim {
        document: document.to_owned(),
        path: path.to_owned(),
    }
}

/// The claims on the object of `bytes`, as its record holds them.
async fn claims_on(store: &MongoStore, bytes: &[u8]) -> Vec<Claim> {
    let record = object_record(store, bytes).await.unwrap();
    let object: StoredObject = bson::deserialize_from_document(record).unwrap();
    object.embedded_by
}

/// Gets the document at `path` from `from` in `form` and puts those bytes at
/// `to` in `into`.
async fn copy(
    from: &AnyStore<'_>,
    path: &StorePath,
    into: &AnyStore<'_>,
    to: &StorePath,
    form: Form,
) {
    let options = WriteOptions::default();
    let bundle = get(from, path, Missing::Keep).await.unwrap();
    let bytes = encode(&bundle, form, &options).unwrap();
    put(
        into,
        to,
        form,
        &bytes,
        &options,
        &Precondition::None,
        Missing::Keep,
    )
    .await
    .unwrap();
}

/// The bucket `just test-stores` makes, its keys under `mongo/<name>`.
fn files_of(name: &str) -> ObjectStore<MinioBucket> {
    let config = ObjectConfig::new(
        env::var("ARCHIE_TEST_MINIO_ENDPOINT").unwrap(),
        "us-east-1".to_owned(),
        env::var("ARCHIE_TEST_MINIO_BUCKET").unwrap(),
        Some(path(&format!("mongo/{name}"))),
    );
    ObjectStore::open(&RealSystem::new(), config).unwrap()
}

/// Runs `scenario` over the collection `files` of the database `name`, its
/// files under `mongo/<name>` in the bucket, then drops the database.
async fn in_database<S>(name: &str, scenario: S)
where
    S: AsyncFnOnce(MongoStore),
{
    let store = MongoStore::open(&uri(), name, "files", files_of(name))
        .await
        .unwrap();
    scenario(store).await;
    let client = Client::with_uri_str(uri()).await.unwrap();
    client.database(name).drop().await.unwrap();
}

/// Each index of `collection`: its keys and whether it is unique, sorted by
/// name.
async fn indexes(collection: &Collection<Document>) -> Vec<(String, Document, bool)> {
    let mut indexes = collection.list_indexes().await.unwrap();
    let mut found = Vec::new();
    while indexes.advance().await.unwrap() {
        let index = indexes.deserialize_current().unwrap();
        let options = index.options.unwrap_or_default();
        found.push((
            options.name.unwrap_or_default(),
            index.keys,
            options.unique == Some(true),
        ));
    }
    found.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    found
}

fn names(paths: &[StorePath]) -> Vec<&str> {
    paths.iter().map(StorePath::as_str).collect()
}

/// A file whose frontmatter holds `count` mappings, each inside the one before.
fn nested_mappings(count: usize) -> RawDocument {
    let mut value = Value::Null(());
    for _ in 0..count {
        value = Value::Mapping(single(value));
    }
    let mut document = parse("# Notes\n").document;
    if let RawDocument::Valid(valid) = &mut document {
        valid.frontmatter = Some(single(value));
    }
    document
}

/// A paragraph inside `count` list items, each inside the one before.
fn nested_items(count: usize) -> String {
    let mut text = String::new();
    for level in 0..count {
        text.push_str(&"  ".repeat(level));
        text.push_str("- item\n");
    }
    text
}

/// The path of the object of `bytes` in the bucket of the store's files.
fn object_of(bytes: &[u8]) -> StorePath {
    path(&format!("files/{}", file::sha256(bytes)))
}

/// The record of the object of `bytes`, as `files_files` holds it.
async fn object_record(store: &MongoStore, bytes: &[u8]) -> Option<Document> {
    let on_id = doc! { "_id": file::sha256(bytes) };
    store.objects.find_one(on_id).await.unwrap()
}

fn path(text: &str) -> StorePath {
    StorePath::new(text).unwrap()
}

/// The paths of `found`, sorted.
fn paths_of(found: &[StoredDocument]) -> Vec<&str> {
    let mut paths: Vec<&str> = found.iter().map(|stored| stored.path.as_str()).collect();
    paths.sort_unstable();
    paths
}

/// The tree `document` keeps once stored: each node's `kind` left out.
fn plain_tree(document: &RawDocument) -> RawDocument {
    from_json(&to_json(document).unwrap()).unwrap()
}

/// `REPORT` with `picture` at `media/logo.png`.
fn report_with(picture: &[u8]) -> BundledDocument {
    let files = BTreeMap::from([("media/logo.png".to_owned(), picture.to_vec())]);
    BundledDocument::new(parse(REPORT).document, files, Missing::Refuse).unwrap()
}

/// Puts `bundle` at `at` in `store`, replacing any document there.
async fn save(store: &MongoStore, at: &str, bundle: &BundledDocument) {
    let options = WriteOptions::default();
    put_bundle(store, &path(at), bundle, &options, &Precondition::None)
        .await
        .unwrap();
}

/// A mapping of the one key `a`, to `value`.
fn single(value: Value) -> Mapping {
    Mapping::from([("a".to_owned(), value)])
}

/// The connection string `just test-stores` sets.
fn uri() -> String {
    env::var("ARCHIE_TEST_MONGODB_URI").unwrap()
}

#[tokio::test]
async fn the_sample_reads_back_finds_and_refuses_a_second_file_at_a_path() {
    in_database("archie_test_reads_back", async |store| {
        let path = StorePath::new("notes/task.md").unwrap();
        let text = b"---\nstatus: open\n---\n# Task\n";
        let revision = store
            .write(&path, text, &Precondition::Absent)
            .await
            .unwrap();
        assert_eq!(store.read(&path).await.unwrap().bytes, text);
        let open = store
            .find(frontmatter_is("status", &Value::String("open".into())).unwrap())
            .await
            .unwrap();
        assert_eq!(open[0].path, "notes/task.md");
        assert!(matches!(
            store
                .write(&path, b"# Again\n", &Precondition::Absent)
                .await,
            Err(StoreError::Conflict(_))
        ));
        store
            .write(&path, b"# Done\n", &Precondition::Matches(revision))
            .await
            .unwrap();
        let done = store.read(&path).await.unwrap();
        assert_eq!(done.bytes, b"# Done\n");
        assert_eq!(done.revision, revision_of("# Done\n"));
        assert!(store.capabilities().atomic_preconditions);
    })
    .await;
}

#[tokio::test]
async fn each_fixture_through_mongodb_and_back_is_archies_form_and_a_second_copy_changes_nothing() {
    in_database("archie_test_copies", async |store| {
        let mongo = AnyStore::Mongo(store);
        let system = MemorySystem::new();
        let files = AnyStore::File(FileStore::new(PathBuf::from("/vault"), &system));
        for (index, source) in FIXTURES.iter().enumerate() {
            let fixture = StorePath::new(&format!("fixtures/{index}.md")).unwrap();
            files
                .write(&fixture, source.as_bytes(), &Precondition::None)
                .await
                .unwrap();
            let expected = back_from_mongodb(source);
            for form in FORMS {
                let first = StorePath::new(&format!("{form:?}/{index}.md")).unwrap();
                let second = StorePath::new(&format!("{form:?}/again/{index}.md")).unwrap();
                copy(&files, &fixture, &mongo, &first, form).await;
                copy(&mongo, &first, &files, &first, form).await;
                let back = files.read(&first).await.unwrap();
                assert_eq!(back.bytes, expected.as_bytes(), "{form:?}: {source}");
                copy(&files, &first, &mongo, &second, form).await;
                copy(&mongo, &second, &files, &second, form).await;
                assert_eq!(
                    files.read(&second).await.unwrap(),
                    back,
                    "{form:?}: {source}"
                );
                let stored = mongo.read(&second).await.unwrap().bytes;
                assert_eq!(stored, back.bytes, "{form:?}: {source}");
            }
        }
    })
    .await;
}

#[tokio::test]
async fn a_precondition_that_does_not_hold_is_a_conflict_and_changes_nothing() {
    in_database("archie_test_preconditions", async |store| {
        let path = StorePath::new("notes/task.md").unwrap();
        let first = store
            .write(&path, b"# Task\n", &Precondition::Absent)
            .await
            .unwrap();
        let second = store
            .write(&path, b"# Done\n", &Precondition::Matches(first.clone()))
            .await
            .unwrap();
        let kept = store.read(&path).await.unwrap();
        assert_eq!(kept.revision, second);
        let conflict = StoreError::Conflict(path.clone());
        let absent = store.write(&path, b"# B\n", &Precondition::Absent).await;
        assert_eq!(absent, Err(conflict.clone()));
        let stale = Precondition::Matches(first);
        assert_eq!(
            store.write(&path, b"# B\n", &stale).await,
            Err(conflict.clone())
        );
        assert_eq!(store.delete(&path, &stale).await, Err(conflict.clone()));
        assert_eq!(
            store.delete(&path, &Precondition::Absent).await,
            Err(conflict)
        );
        assert_eq!(store.read(&path).await.unwrap(), kept);
        assert_eq!(store.find(Document::new()).await.unwrap().len(), 1);

        store
            .delete(&path, &Precondition::Matches(second))
            .await
            .unwrap();
        let missing = StoreError::NotFound(path.clone());
        assert_eq!(store.read(&path).await, Err(missing.clone()));
        assert_eq!(store.read_document(&path).await, Err(missing.clone()));
        assert_eq!(
            store.delete(&path, &Precondition::None).await,
            Err(missing.clone())
        );
        assert_eq!(
            store.delete(&path, &Precondition::Absent).await,
            Err(missing)
        );
        let gone = Precondition::Matches(kept.revision);
        let unmatched = store.write(&path, b"# C\n", &gone).await;
        assert_eq!(unmatched, Err(StoreError::Conflict(path.clone())));
        assert_eq!(store.list(None).await, Ok(Vec::new()));
    })
    .await;
}

#[tokio::test]
async fn a_write_with_no_precondition_inserts_then_replaces_the_one_record_at_the_path() {
    in_database("archie_test_upserts", async |store| {
        let path = StorePath::new("notes/task.md").unwrap();
        store
            .write(&path, b"# Task\n", &Precondition::None)
            .await
            .unwrap();
        let revision = store
            .write(&path, b"# Done\n", &Precondition::None)
            .await
            .unwrap();
        assert_eq!(revision, revision_of("# Done\n"));
        let all = store.find(Document::new()).await.unwrap();
        assert_eq!(paths_of(&all), ["notes/task.md"]);
        assert_eq!(all[0].revision, revision.0);
        assert_eq!(store.read(&path).await.unwrap().bytes, b"# Done\n");
        store.delete(&path, &Precondition::None).await.unwrap();
        assert_eq!(store.list(None).await, Ok(Vec::new()));
    })
    .await;
}

#[tokio::test]
async fn a_query_finds_the_files_by_their_frontmatter_and_compares_dates_as_dates() {
    in_database("archie_test_queries", async |store| {
        let files = [
            (
                "notes/a.md",
                "---\nstatus: open\ndue: 2026-10-20\n---\n# A\n",
            ),
            (
                "notes/b.md",
                "---\nstatus: done\ndue: 2026-12-01\n---\n# B\n",
            ),
            (
                "notes/c.md",
                "---\nstatus: open\ndue: 2026-11-15\n---\n# C\n",
            ),
            ("notes/d.md", "# D\n"),
        ];
        for (path, text) in files {
            let at = StorePath::new(path).unwrap();
            store
                .write(&at, text.as_bytes(), &Precondition::None)
                .await
                .unwrap();
        }
        let open = frontmatter_is("status", &Value::String("open".into())).unwrap();
        let opened = store.find(open).await.unwrap();
        assert_eq!(paths_of(&opened), ["notes/a.md", "notes/c.md"]);
        let november = DateTime::parse_rfc3339_str("2026-11-01T00:00:00Z").unwrap();
        let before = doc! { "doc.data.due": { "$lt": november } };
        let due = store.find(before).await.unwrap();
        assert_eq!(paths_of(&due), ["notes/a.md"]);
        let archie = write(&parse(files[0].1).document, &WriteOptions::default()).unwrap();
        assert_eq!(due[0].revision, revision_of(&archie).0);
    })
    .await;
}

#[tokio::test]
async fn a_query_finds_a_frontmatter_key_holding_a_dot_by_its_whole_name() {
    in_database("archie_test_dotted_keys", async |store| {
        let files = [
            ("notes/dotted.md", "---\na.b: x\n---\n# Dotted\n"),
            ("notes/nested.md", "---\na:\n  b: x\n---\n# Nested\n"),
            ("notes/status.md", "---\nstatus: open\n---\n# Status\n"),
        ];
        for (path, text) in files {
            let at = StorePath::new(path).unwrap();
            store
                .write(&at, text.as_bytes(), &Precondition::None)
                .await
                .unwrap();
        }
        let dotted = frontmatter_is("a.b", &Value::String("x".into())).unwrap();
        assert_eq!(
            paths_of(&store.find(dotted).await.unwrap()),
            ["notes/dotted.md"]
        );
        let open = frontmatter_is("status", &Value::String("open".into())).unwrap();
        assert_eq!(
            paths_of(&store.find(open).await.unwrap()),
            ["notes/status.md"]
        );
    })
    .await;
}

#[tokio::test]
async fn a_query_finds_a_frontmatter_key_starting_with_a_dollar_by_its_name() {
    in_database("archie_test_dollar_keys", async |store| {
        let at = StorePath::new("notes/price.md").unwrap();
        store
            .write(&at, b"---\n$price: 3\n---\n", &Precondition::None)
            .await
            .unwrap();
        let price = frontmatter_is("$price", &Value::Integer(3)).unwrap();
        assert_eq!(
            paths_of(&store.find(price).await.unwrap()),
            ["notes/price.md"]
        );
    })
    .await;
}

#[tokio::test]
async fn a_document_of_13_nested_list_items_fails_the_write_naming_the_path_and_the_depth() {
    in_database("archie_test_depth", async |store| {
        let path = StorePath::new("notes/deep.md").unwrap();
        let refused = "notes/deep.md nests 102 levels, and MongoDB holds 100";
        let deep = nested_items(13);
        for precondition in [Precondition::None, Precondition::Absent] {
            let written = store.write(&path, deep.as_bytes(), &precondition).await;
            assert_eq!(written, Err(StoreError::Backend(refused.to_owned())));
        }
        assert_eq!(
            store.read(&path).await,
            Err(StoreError::NotFound(path.clone()))
        );
        assert_eq!(store.list(None).await, Ok(Vec::new()));

        let twelve = nested_items(12);
        store
            .write(&path, twelve.as_bytes(), &Precondition::None)
            .await
            .unwrap();
        assert_eq!(store.read(&path).await.unwrap().bytes, twelve.as_bytes());
    })
    .await;
}

#[tokio::test]
async fn a_record_100_levels_deep_is_stored_and_one_101_deep_is_not() {
    in_database("archie_test_hundred_levels", async |store| {
        let path = StorePath::new("notes/notes.md").unwrap();
        let hundred = nested_mappings(97);
        let revision = store
            .write_document(&path, &hundred, &Precondition::Absent)
            .await
            .unwrap();
        let text = write(&hundred, &WriteOptions::default()).unwrap();
        assert_eq!(revision, revision_of(&text));
        let read = store.read_document(&path).await.unwrap();
        assert_eq!(read, (plain_tree(&hundred), revision));

        let other = StorePath::new("notes/other.md").unwrap();
        let refused = "notes/other.md nests 101 levels, and MongoDB holds 100";
        let written = store
            .write_document(&other, &nested_mappings(98), &Precondition::None)
            .await;
        assert_eq!(written, Err(StoreError::Backend(refused.to_owned())));
        assert_eq!(names(&store.list(None).await.unwrap()), ["notes/notes.md"]);
    })
    .await;
}

#[tokio::test]
async fn a_write_of_bytes_that_are_not_utf8_names_the_path_and_the_offset_and_stores_nothing() {
    in_database("archie_test_not_utf8", async |store| {
        let path = StorePath::new("notes/a.md").unwrap();
        let refused = "notes/a.md: not UTF-8 at byte 1";
        let written = store.write(&path, b"a\xffb", &Precondition::None).await;
        assert_eq!(written, Err(StoreError::Backend(refused.to_owned())));
        assert_eq!(store.list(None).await, Ok(Vec::new()));
    })
    .await;
}

#[tokio::test]
async fn a_listing_gives_the_paths_under_the_prefix_sorted_by_their_bytes() {
    in_database("archie_test_listing", async |store| {
        for path in ["notesx/a.md", "notes/b/c.md", "notes/a.md", "no.es/d.md"] {
            let at = StorePath::new(path).unwrap();
            store
                .write(&at, b"# File\n", &Precondition::None)
                .await
                .unwrap();
        }
        let notes = StorePath::new("notes").unwrap();
        let in_notes = store.list(Some(&notes)).await.unwrap();
        assert_eq!(names(&in_notes), ["notes/a.md", "notes/b/c.md"]);
        let dotted = StorePath::new("no.es").unwrap();
        let in_dotted = store.list(Some(&dotted)).await.unwrap();
        assert_eq!(names(&in_dotted), ["no.es/d.md"]);
        let all = store.list(None).await.unwrap();
        assert_eq!(
            names(&all),
            ["no.es/d.md", "notes/a.md", "notes/b/c.md", "notesx/a.md"]
        );
    })
    .await;
}

#[tokio::test]
async fn open_makes_the_unique_index_on_path_when_the_collection_has_none() {
    in_database("archie_test_index", async |store| {
        let made = [
            ("_id_".to_owned(), doc! { "_id": 1_i32 }, false),
            ("path_1".to_owned(), doc! { "path": 1_i32 }, true),
        ];
        assert_eq!(indexes(&store.collection).await, made);
        let again = MongoStore::open(
            &uri(),
            "archie_test_index",
            "files",
            files_of("archie_test_index"),
        )
        .await
        .unwrap();
        assert_eq!(indexes(&again.collection).await, made);

        let client = Client::with_uri_str(uri()).await.unwrap();
        let named: Collection<Document> = client.database("archie_test_index").collection("named");
        let by_path = IndexModel::builder()
            .keys(doc! { "path": -1_i32 })
            .options(
                IndexOptions::builder()
                    .name("by_path".to_owned())
                    .unique(true)
                    .build(),
            )
            .build();
        named.create_index(by_path).await.unwrap();
        let held = MongoStore::open(
            &uri(),
            "archie_test_index",
            "named",
            files_of("archie_test_index"),
        )
        .await
        .unwrap();
        let kept = [
            ("_id_".to_owned(), doc! { "_id": 1_i32 }, false),
            ("by_path".to_owned(), doc! { "path": -1_i32 }, true),
        ];
        assert_eq!(indexes(&held.collection).await, kept);
    })
    .await;
}

#[tokio::test]
async fn each_call_to_any_store_goes_to_the_mongodb_store_it_holds() {
    in_database("archie_test_any_store", async |store| {
        let any = AnyStore::Mongo(store);
        let path = StorePath::new("notes/a.md").unwrap();
        assert!(any.capabilities().atomic_preconditions);
        let options = WriteOptions::default();
        let revision = put(
            &any,
            &path,
            Form::Markdown,
            b"* item\n",
            &options,
            &Precondition::Absent,
            Missing::Keep,
        )
        .await
        .unwrap();
        assert_eq!(any.read(&path).await.unwrap().revision, revision);
        let bundle = get(&any, &path, Missing::Keep).await.unwrap();
        let markdown = encode(&bundle, Form::Markdown, &options).unwrap();
        assert_eq!(markdown, b"- item\n");
        assert_eq!(names(&any.list(None).await.unwrap()), ["notes/a.md"]);
        any.delete(&path, &Precondition::Matches(revision))
            .await
            .unwrap();
        assert_eq!(any.list(None).await, Ok(Vec::new()));
    })
    .await;
}

#[tokio::test]
async fn open_makes_the_index_on_embedded_by_path_of_the_files_collection_when_it_has_none() {
    in_database("archie_test_files_index", async |store| {
        let made = [
            ("_id_".to_owned(), doc! { "_id": 1_i32 }, false),
            (
                "embedded_by.path_1".to_owned(),
                doc! { "embedded_by.path": 1_i32 },
                false,
            ),
        ];
        assert_eq!(indexes(&store.objects).await, made);
        let again = MongoStore::open(
            &uri(),
            "archie_test_files_index",
            "files",
            files_of("archie_test_files_index"),
        )
        .await
        .unwrap();
        assert_eq!(indexes(&again.objects).await, made);

        let client = Client::with_uri_str(uri()).await.unwrap();
        let named: Collection<Document> = client
            .database("archie_test_files_index")
            .collection("named_files");
        let by_path = IndexModel::builder()
            .keys(doc! { "embedded_by.path": -1_i32 })
            .options(IndexOptions::builder().name("by_path".to_owned()).build())
            .build();
        named.create_index(by_path).await.unwrap();
        let held = MongoStore::open(
            &uri(),
            "archie_test_files_index",
            "named",
            files_of("archie_test_files_index"),
        )
        .await
        .unwrap();
        let kept = [
            ("_id_".to_owned(), doc! { "_id": 1_i32 }, false),
            (
                "by_path".to_owned(),
                doc! { "embedded_by.path": -1_i32 },
                false,
            ),
        ];
        assert_eq!(indexes(&held.objects).await, kept);
    })
    .await;
}

#[tokio::test]
async fn a_save_stores_the_object_its_record_and_the_files_map_and_a_get_gives_each_file_back() {
    in_database("archie_test_save", async |store| {
        let bundle = report_with(PNG);
        save(&store, "notes/report.md", &bundle).await;
        let hash = file::sha256(PNG);
        let object = object_of(PNG);
        assert_eq!(store.files.read(&object).await.unwrap().bytes, PNG);
        let record = doc! {
            "_id": hash.as_str(),
            "object": object.as_str(),
            "content_type": "image/png",
            "size": 27_i64,
            "description": "Logo",
            "stored": true,
            "embedded_by": [{ "document": "notes/report.md", "path": "notes/media/logo.png" }],
        };
        assert_eq!(object_record(&store, PNG).await, Some(record));
        let documents = store.find(Document::new()).await.unwrap();
        let files = BTreeMap::from([("media/logo.png".to_owned(), hash)]);
        assert_eq!(documents[0].files, files);
        let report = path("notes/report.md");
        assert_eq!(get(&store, &report, Missing::Refuse).await.unwrap(), bundle);
    })
    .await;
}

#[tokio::test]
async fn a_second_save_of_the_same_document_claims_its_object_once_and_uploads_nothing() {
    in_database("archie_test_save_twice", async |store| {
        save(&store, "notes/report.md", &report_with(PNG)).await;
        let object = object_of(PNG);
        // With the object gone, only an upload could put it back.
        store
            .files
            .delete(&object, &Precondition::None)
            .await
            .unwrap();
        save(&store, "notes/report.md", &report_with(PNG)).await;
        let claims = [claim("notes/report.md", "notes/media/logo.png")];
        assert_eq!(claims_on(&store, PNG).await, claims);
        let record = object_record(&store, PNG).await.unwrap();
        assert!(record.get_bool("stored").unwrap());
        let missing = StoreError::NotFound(object.clone());
        assert_eq!(store.files.read(&object).await, Err(missing));
    })
    .await;
}

#[tokio::test]
async fn two_documents_embedding_the_same_bytes_make_one_object_and_one_record_with_two_claims() {
    in_database("archie_test_shared", async |store| {
        save(&store, "notes/report.md", &report_with(PNG)).await;
        save(&store, "drafts/report.md", &report_with(PNG)).await;
        let records = store.objects.count_documents(Document::new()).await;
        assert_eq!(records.unwrap(), 1);
        let claims = [
            claim("notes/report.md", "notes/media/logo.png"),
            claim("drafts/report.md", "drafts/media/logo.png"),
        ];
        assert_eq!(claims_on(&store, PNG).await, claims);
        assert_eq!(store.files.list(None).await.unwrap(), [object_of(PNG)]);
    })
    .await;
}

#[tokio::test]
async fn a_save_that_replaces_a_picture_releases_the_old_object_and_keeps_it_in_the_bucket() {
    in_database("archie_test_replace", async |store| {
        save(&store, "notes/report.md", &report_with(PNG)).await;
        save(&store, "notes/report.md", &report_with(OTHER_PNG)).await;
        let claims = [claim("notes/report.md", "notes/media/logo.png")];
        assert_eq!(claims_on(&store, OTHER_PNG).await, claims);
        assert_eq!(claims_on(&store, PNG).await, []);
        for bytes in [PNG, OTHER_PNG] {
            let object = store.files.read(&object_of(bytes)).await.unwrap();
            assert_eq!(object.bytes, bytes);
        }
        let report = path("notes/report.md");
        let back = get(&store, &report, Missing::Refuse).await.unwrap();
        assert_eq!(back, report_with(OTHER_PNG));
    })
    .await;
}

#[tokio::test]
async fn each_step_of_a_save_that_replaces_a_picture_leaves_the_file_reading_one_whole_version() {
    in_database("archie_test_replace_steps", async |store| {
        let report = path("notes/report.md");
        let logo = path("notes/media/logo.png");
        save(&store, "notes/report.md", &report_with(PNG)).await;
        let document = parse(REPORT).document;
        let files = BTreeMap::from([(logo.clone(), OTHER_PNG.to_vec())]);
        let replaced = embedded(&report, &document, &files).unwrap();
        let picture = &replaced[0];
        let claims = [claim("notes/report.md", "notes/media/logo.png")];

        store.claim(&report, picture).await.unwrap();
        assert_eq!(claims_on(&store, OTHER_PNG).await, claims);
        let record = object_record(&store, OTHER_PNG).await.unwrap();
        assert!(!record.get_bool("stored").unwrap());
        let missing = StoreError::NotFound(picture.object.clone());
        assert_eq!(store.files.read(&picture.object).await, Err(missing));
        assert_eq!(store.read(&logo).await.unwrap().bytes, PNG);

        store.upload(picture).await.unwrap();
        let uploaded = store.files.read(&picture.object).await.unwrap();
        assert_eq!(uploaded.bytes, OTHER_PNG);
        assert_eq!(store.read(&logo).await.unwrap().bytes, PNG);

        let hashes = BTreeMap::from([("media/logo.png".to_owned(), picture.hash.clone())]);
        store
            .save(&report, &document, &hashes, &Precondition::None)
            .await
            .unwrap();
        assert_eq!(claims_on(&store, PNG).await, claims);
        assert_eq!(store.read(&logo).await.unwrap().bytes, OTHER_PNG);

        store.release(&report, &hashes).await.unwrap();
        assert_eq!(claims_on(&store, PNG).await, []);
        assert_eq!(claims_on(&store, OTHER_PNG).await, claims);
        assert_eq!(store.read(&logo).await.unwrap().bytes, OTHER_PNG);
    })
    .await;
}

#[tokio::test]
async fn a_read_of_a_files_own_path_gives_the_object_the_record_of_its_document_names() {
    in_database("archie_test_file_path", async |store| {
        save(&store, "notes/report.md", &report_with(PNG)).await;
        let draft = path("notes/draft.md");
        let files = BTreeMap::from([(path("notes/media/logo.png"), OTHER_PNG.to_vec())]);
        let unsaved = embedded(&draft, &parse(REPORT).document, &files).unwrap();
        // A claim whose document was never saved names no object.
        store.claim(&draft, &unsaved[0]).await.unwrap();
        let logo = path("notes/media/logo.png");
        let read = store.read(&logo).await.unwrap();
        let expected = Stored {
            bytes: PNG.to_vec(),
            revision: file::revision(PNG),
        };
        assert_eq!(read, expected);
        let other = path("notes/media/other.png");
        let missing = StoreError::NotFound(other.clone());
        assert_eq!(store.read(&other).await, Err(missing));
    })
    .await;
}

#[tokio::test]
async fn a_record_written_with_no_files_reads_as_a_document_with_no_files() {
    in_database("archie_test_no_files", async |store| {
        let report = path("notes/report.md");
        let mut record = to_record(&StoredDocument {
            document: parse(REPORT).document,
            files: BTreeMap::new(),
            id: ObjectId::new(),
            path: report.as_str().to_owned(),
            revision: revision_of(REPORT).0,
        })
        .unwrap();
        record.remove("files");
        store.collection.insert_one(record).await.unwrap();
        let bundle = get(&store, &report, Missing::Keep).await.unwrap();
        assert_eq!(bundle.files(), Vec::<&str>::new());
        let logo = path("notes/media/logo.png");
        let missing = StoreError::NotFound(logo.clone());
        assert_eq!(store.read_embedded(&report, &logo).await, Err(missing));
    })
    .await;
}

#[tokio::test]
async fn a_claim_while_a_deletion_holds_the_record_is_refused_and_changes_nothing() {
    in_database("archie_test_claim_held", async |store| {
        let report = path("notes/report.md");
        let files = BTreeMap::from([(path("notes/media/logo.png"), PNG.to_vec())]);
        let picture = embedded(&report, &parse(REPORT).document, &files).unwrap();
        let held = doc! {
            "_id": picture[0].hash.as_str(),
            "object": picture[0].object.as_str(),
            "content_type": "image/png",
            "size": 27_i64,
            "description": "Logo",
            "stored": true,
            "deleting": true,
            "embedded_by": [],
        };
        store.objects.insert_one(held.clone()).await.unwrap();
        assert_eq!(store.try_claim(&report, &picture[0]).await, Ok(false));
        assert_eq!(object_record(&store, PNG).await, Some(held));
    })
    .await;
}

#[tokio::test]
async fn a_file_outside_the_folder_of_its_document_fails_the_save_naming_it_and_claims_nothing() {
    in_database("archie_test_outside", async |store| {
        let report = path("notes/report.md");
        let files = BTreeMap::from([(path("other/logo.png"), PNG.to_vec())]);
        let written = store
            .write_with_embedded(&report, REPORT, &files, &Precondition::None)
            .await;
        let refused = "other/logo.png: not in the folder of notes/report.md";
        assert_eq!(written, Err(StoreError::Backend(refused.to_owned())));
        let records = store.objects.count_documents(Document::new()).await;
        assert_eq!(records.unwrap(), 0);
        assert_eq!(store.list(None).await, Ok(Vec::new()));
    })
    .await;
}
