//! The store against the MongoDB and `RustFS` servers `just test-stores`
//! starts, each test, or each lane of a test that runs every order of its
//! steps, in a database of its own, dropped when it ends, and its files under a
//! key prefix of its own.

use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use core::mem;
use std::path::PathBuf;
use std::{env, thread};

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
use tokio::runtime::Builder;

use super::{MongoStore, embedded, object_path, stopped_deletion, stored_release_failed};
use crate::any::AnyStore;
use crate::document::{get, put, put_bundle};
use crate::file::{self, FileStore};
use crate::mongo::record::{
    Claim, StoredDocument, StoredObject, from_record, frontmatter_is, revision_of, to_record,
};
use crate::object::{ObjectConfig, ObjectStore};
use crate::{Precondition, Store as _, StoreError, StorePath, Stored};

const COPY_MD: &str = "notes/copy.md";

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

const INTRO: &str = "Intro ![chart](../media/chart.png)\n";

const LOGO: &str = "media/logo.png";

const NEW_MD: &str = "notes/new.md";

const OLD_MD: &str = "notes/old.md";

/// Each lane holds a client and a database of its own, so few lanes run at once.
const ORDERS_AT_ONCE: usize = 8;

const OTHER_MD: &str = "notes/other.md";

const OTHER_PNG: &[u8] = b"\x89PNG\r\n\x1a\n another picture";

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n not a real picture";

const REPORT: &str = "# Report\n\n![Logo](media/logo.png)\n";

const REPORT_MD: &str = "notes/report.md";

const REPORT_WITH_INTRO: &str = "# Report\n\n![[parts/intro]]\n";

static ONE_SAVE: Scenario = Scenario {
    actors: &[
        Actor {
            name: "the save of notes/new.md",
            steps: &saving(NEW_MD, PNG),
        },
        Actor {
            name: "the deletion of the logo",
            steps: &deleting(PNG),
        },
    ],
    documents: &[(NEW_MD, Some(PNG)), (OLD_MD, None)],
    leftover: &[],
    start: Start::Unembedded(PNG),
};

static TWO_SAVES: Scenario = Scenario {
    actors: &[
        Actor {
            name: "the save of notes/new.md",
            steps: &saving(NEW_MD, PNG),
        },
        Actor {
            name: "the save of notes/other.md",
            steps: &saving(OTHER_MD, PNG),
        },
        Actor {
            name: "the deletion of the logo",
            steps: &deleting(PNG),
        },
    ],
    documents: &[(NEW_MD, Some(PNG)), (OLD_MD, None), (OTHER_MD, Some(PNG))],
    leftover: &[],
    start: Start::Unembedded(PNG),
};

static A_REPLACEMENT: Scenario = Scenario {
    actors: &[
        Actor {
            name: "the save of notes/report.md",
            steps: &[
                Step::Claim(REPORT_MD, OTHER_PNG),
                Step::Upload(REPORT_MD, OTHER_PNG),
                Step::Save(REPORT_MD, OTHER_PNG),
                Step::Release(REPORT_MD, Some(OTHER_PNG)),
                Step::Mark(None),
                Step::RemoveObject,
                Step::RemoveRecord,
            ],
        },
        Actor {
            name: "the deletion of the old logo",
            steps: &deleting(PNG),
        },
    ],
    documents: &[(REPORT_MD, Some(OTHER_PNG))],
    leftover: &[],
    start: Start::Embedded(PNG),
};

static A_DOCUMENT_DELETION: Scenario = Scenario {
    actors: &[
        Actor {
            name: "the deletion of notes/report.md",
            steps: &[
                Step::RemoveDocument(REPORT_MD),
                Step::Release(REPORT_MD, None),
                Step::Mark(None),
                Step::RemoveObject,
                Step::RemoveRecord,
            ],
        },
        Actor {
            name: "the save of notes/copy.md",
            steps: &saving(COPY_MD, PNG),
        },
    ],
    documents: &[(COPY_MD, Some(PNG))],
    leftover: &[],
    start: Start::Embedded(PNG),
};

static A_SAVE_CUT_AFTER_ITS_CLAIM: Scenario = Scenario {
    actors: &[
        Actor {
            name: "the save of notes/new.md",
            steps: &[Step::Claim(NEW_MD, PNG)],
        },
        Actor {
            name: "the deletion of the logo",
            steps: &deleting(PNG),
        },
    ],
    documents: &[(OLD_MD, None)],
    leftover: &[(NEW_MD, PNG)],
    start: Start::Unembedded(PNG),
};

static A_SAVE_CUT_AFTER_ITS_UPLOAD: Scenario = Scenario {
    actors: &[
        Actor {
            name: "the save of notes/new.md",
            steps: &[Step::Claim(NEW_MD, PNG), Step::Upload(NEW_MD, PNG)],
        },
        Actor {
            name: "the deletion of the logo",
            steps: &deleting(PNG),
        },
    ],
    documents: &[(OLD_MD, None)],
    leftover: &[(NEW_MD, PNG)],
    start: Start::Unembedded(PNG),
};

static TWO_DELETIONS: Scenario = Scenario {
    actors: &[
        Actor {
            name: "a first deletion of the logo",
            steps: &deleting(PNG),
        },
        Actor {
            name: "a second deletion of the logo",
            steps: &deleting(PNG),
        },
        Actor {
            name: "the save of notes/new.md",
            steps: &saving(NEW_MD, PNG),
        },
    ],
    documents: &[(NEW_MD, Some(PNG)), (OLD_MD, None)],
    leftover: &[],
    start: Start::Unembedded(PNG),
};

/// One actor of a scenario: its name in a failing order's message, and its
/// steps, in their sequence.
struct Actor {
    name: &'static str,
    steps: &'static [Step],
}

/// What a store holds: its documents, the records of its objects, and the
/// objects in its bucket.
struct Holdings {
    documents: Vec<StoredDocument>,
    objects: Vec<StorePath>,
    records: Vec<StoredObject>,
}

impl Holdings {
    async fn of(store: &MongoStore) -> Self {
        let mut found = store.objects.find(Document::new()).await.unwrap();
        let mut records = Vec::new();
        while found.advance().await.unwrap() {
            let record = found.deserialize_current().unwrap();
            records.push(bson::deserialize_from_document(record).unwrap());
        }
        Self {
            documents: store.find(Document::new()).await.unwrap(),
            objects: store.files.list(None).await.unwrap(),
            records,
        }
    }
}

/// Where one actor is in an order: its next step, the objects its deletion
/// steps act on, whether its deletion found nothing to mark, and whether its
/// claim waits for a deletion.
#[derive(Default)]
struct Run {
    held: Vec<String>,
    next: usize,
    stopped: bool,
    waiting: bool,
}

/// Actors that take their steps in each order: what the store holds before
/// the first step, each document and the picture it embeds once every actor
/// finishes, and each claim nothing uses that stays, by its document and
/// picture.
struct Scenario {
    actors: &'static [Actor],
    documents: &'static [(&'static str, Option<&'static [u8]>)],
    leftover: &'static [(&'static str, &'static [u8])],
    start: Start,
}

/// What a scenario's store holds before the first step: `REPORT_MD`
/// embedding a picture, or `OLD_MD` that embedded it and stopped, the record
/// of the picture holding no claim and no deletion of it run.
#[derive(Clone, Copy)]
enum Start {
    Embedded(&'static [u8]),
    Unembedded(&'static [u8]),
}

/// One step of an actor, named for the store's function it runs, with the
/// document and the picture it acts on. `Mark` marks the object of its
/// picture, or with none each object the actor's release took a claim from;
/// `RemoveObject` and `RemoveRecord` act on what it marked.
#[derive(Clone, Copy)]
enum Step {
    Claim(&'static str, &'static [u8]),
    Mark(Option<&'static [u8]>),
    Release(&'static str, Option<&'static [u8]>),
    RemoveDocument(&'static str),
    RemoveObject,
    RemoveRecord,
    Save(&'static str, &'static [u8]),
    Upload(&'static str, &'static [u8]),
}

impl Step {
    const fn name(self) -> &'static str {
        match self {
            Self::Claim(..) => "claim",
            Self::Mark(_) => "mark",
            Self::Release(..) => "release",
            Self::RemoveDocument(_) => "remove_document",
            Self::RemoveObject => "remove_object",
            Self::RemoveRecord => "remove_record",
            Self::Save(..) => "save",
            Self::Upload(..) => "upload",
        }
    }
}

/// Asserts what must hold once every actor of `scenario` finishes: what each
/// step must leave, the documents it ends with, a read of each picture they
/// embed, no record held by a deletion, and each record's claims exactly
/// those the documents' files make, with the claims nothing uses it leaves.
async fn assert_finished(store: &MongoStore, scenario: &Scenario, told: &str) {
    let held = Holdings::of(store).await;
    assert_readable(&held, told);
    let ended: BTreeMap<&str, BTreeMap<String, String>> = held
        .documents
        .iter()
        .map(|stored| (stored.path.as_str(), stored.files.clone()))
        .collect();
    let expected: BTreeMap<&str, BTreeMap<String, String>> = scenario
        .documents
        .iter()
        .map(|&(document, picture)| (document, logo_of(picture)))
        .collect();
    assert_eq!(ended, expected, "{told}: the documents");
    let mut made: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    for stored in &held.documents {
        let at = path(&stored.path);
        for (name, hash) in &stored.files {
            let file = at.beside(name).unwrap();
            let claims = made.entry(hash.clone()).or_default();
            claims.insert((stored.path.clone(), file.as_str().to_owned()));
        }
    }
    for &(document, picture) in scenario.leftover {
        let file = path(document).beside(LOGO).unwrap();
        let claims = made.entry(file::sha256(picture)).or_default();
        claims.insert((document.to_owned(), file.as_str().to_owned()));
    }
    let claimed: BTreeMap<String, Vec<(String, String)>> = made
        .into_iter()
        .map(|(hash, claims)| (hash, claims.into_iter().collect()))
        .collect();
    let recorded: BTreeMap<String, Vec<(String, String)>> = held
        .records
        .iter()
        .map(|record| {
            let mut claims: Vec<(String, String)> = record
                .embedded_by
                .iter()
                .map(|by| (by.document.clone(), by.path.clone()))
                .collect();
            claims.sort_unstable();
            (record.id.clone(), claims)
        })
        .collect();
    assert_eq!(recorded, claimed, "{told}: the claims each record holds");
    assert!(
        held.records.iter().all(|record| !record.deleting),
        "{told}: a deletion still holds a record"
    );
    for &(document, picture) in scenario.documents {
        if let Some(bytes) = picture {
            let at = path(document);
            let read = store.read_embedded(&at, &at.beside(LOGO).unwrap()).await;
            assert_eq!(read, Ok(bytes.to_vec()), "{told}: the logo of {at}");
        }
    }
}

/// Asserts what each step must leave: each file a document names is in the
/// bucket, and its record holds the document's claim; each object in the
/// bucket has a record; and each record that says its object is stored, and
/// that no deletion holds, has its object in the bucket.
fn assert_readable(held: &Holdings, told: &str) {
    for document in &held.documents {
        let at = path(&document.path);
        for (name, hash) in &document.files {
            let object = object_path(hash).unwrap();
            assert!(
                held.objects.contains(&object),
                "{told}: {at} embeds {name}, whose object {object} is gone"
            );
            let made = claim(&document.path, at.beside(name).unwrap().as_str());
            let record = held.records.iter().find(|record| record.id == *hash);
            assert!(
                record.is_some_and(|found| found.embedded_by.contains(&made)),
                "{told}: {at} embeds {name}, and no record of {object} holds its claim"
            );
        }
    }
    for object in &held.objects {
        assert!(
            held.records
                .iter()
                .any(|record| record.object == object.as_str()),
            "{told}: {object} is in the bucket, and no record names it"
        );
    }
    let stored = held
        .records
        .iter()
        .filter(|record| record.stored && !record.deleting);
    for record in stored {
        let object = object_path(&record.id).unwrap();
        assert!(
            held.objects.contains(&object),
            "{told}: the record of {object} says it is stored, and the bucket does not hold it"
        );
    }
}

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

/// Puts in `store` what `start` says it holds before the first step.
async fn begin(store: &MongoStore, start: Start) {
    match start {
        Start::Embedded(picture) => save(store, REPORT_MD, &report_with(picture)).await,
        Start::Unembedded(picture) => {
            save(store, OLD_MD, &report_with(picture)).await;
            let old = path(OLD_MD);
            let without = parse("# Report\n").document;
            let none = BTreeMap::new();
            store
                .save(&old, &without, &none, &Precondition::None)
                .await
                .unwrap();
            let released = store.release(&old, &none).await;
            assert_eq!(released, Ok(vec![file::sha256(picture)]));
        }
    }
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

/// The three steps of the deletion of the object of `picture`.
const fn deleting(picture: &'static [u8]) -> [Step; 3] {
    [
        Step::Mark(Some(picture)),
        Step::RemoveObject,
        Step::RemoveRecord,
    ]
}

/// The `description` of the record of the object of `bytes`.
async fn description_of(store: &MongoStore, bytes: &[u8]) -> String {
    let record = object_record(store, bytes).await.unwrap();
    record.get_str("description").unwrap().to_owned()
}

/// Deletes every document, record and object of `store`.
async fn empty(store: &MongoStore) {
    store.collection.delete_many(Document::new()).await.unwrap();
    store.objects.delete_many(Document::new()).await.unwrap();
    for object in store.files.list(None).await.unwrap() {
        store
            .files
            .delete(&object, &Precondition::None)
            .await
            .unwrap();
    }
}

/// Runs `scenario` in each order of its actors' steps, `ORDERS_AT_ONCE`
/// orders at a time, each lane a thread with a store of its own emptied
/// before each of its orders, and gives how many orders ran.
fn every_order(name: &str, scenario: &Scenario) -> usize {
    let counts: Vec<usize> = scenario
        .actors
        .iter()
        .map(|actor| actor.steps.len())
        .collect();
    let orders = interleavings(&counts);
    let width = ORDERS_AT_ONCE.min(orders.len());
    thread::scope(|lanes| {
        for lane in 0..width {
            let taken = orders.iter().skip(lane).step_by(width);
            lanes.spawn(move || {
                let runtime = Builder::new_current_thread().enable_all().build().unwrap();
                runtime.block_on(async {
                    let database = format!("{name}_{lane}");
                    let store = MongoStore::open(&uri(), &database, "files", files_of(&database))
                        .await
                        .unwrap();
                    for order in taken {
                        empty(&store).await;
                        begin(&store, scenario.start).await;
                        run(&store, scenario, order).await;
                    }
                    let client = Client::with_uri_str(uri()).await.unwrap();
                    client.database(&database).drop().await.unwrap();
                });
            });
        }
    });
    orders.len()
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

/// Stops a deletion of the logo `OLD_MD` embedded after its mark, and after
/// its `remove_object` too where `object_removed` says so, then asserts that
/// `finish_deletions` finishes it and that a save of `NEW_MD` embedding the
/// logo stores it again.
async fn finishes_a_stopped_deletion(store: &MongoStore, object_removed: bool) {
    begin(store, Start::Unembedded(PNG)).await;
    let hash = file::sha256(PNG);
    assert_eq!(store.mark(&hash).await, Ok(true));
    if object_removed {
        store.remove_object(&hash).await.unwrap();
    }
    let new = path(NEW_MD);
    let files = logo_files(NEW_MD, PNG);
    let picture = embedded(&new, &parse(REPORT).document, &files).unwrap();
    let logo = &picture[0];
    assert_eq!(store.try_claim(&new, logo).await, Ok(false));

    assert_eq!(store.finish_deletions().await, Ok(vec![hash]));
    assert_eq!(object_record(store, PNG).await, None);
    assert_eq!(store.files.list(None).await, Ok(Vec::new()));

    assert_eq!(store.try_claim(&new, logo).await, Ok(true));
    save(store, NEW_MD, &report_with(PNG)).await;
    let read = store.read_embedded(&new, logo.path).await;
    assert_eq!(read, Ok(PNG.to_vec()));
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

/// Every order of the steps of actors taking `counts[actor]` steps each that
/// keeps each actor's own steps in sequence, an order being the index of the
/// actor of each step.
fn interleavings(counts: &[usize]) -> Vec<Vec<usize>> {
    let mut orders = Vec::new();
    let mut pending = vec![(Vec::new(), counts.to_vec())];
    while let Some((order, left)) = pending.pop() {
        if left.iter().all(|count| *count == 0) {
            orders.push(order);
            continue;
        }
        for (actor, count) in left.iter().enumerate() {
            if *count > 0 {
                let mut longer = order.clone();
                longer.push(actor);
                let mut rest = left.clone();
                rest[actor] -= 1;
                pending.push((longer, rest));
            }
        }
    }
    orders
}

/// `picture` at `LOGO` beside the document at `document`.
fn logo_files(document: &str, picture: &[u8]) -> BTreeMap<StorePath, Vec<u8>> {
    let file = path(document).beside(LOGO).unwrap();
    BTreeMap::from([(file, picture.to_vec())])
}

/// The `files` of the record of a document that embeds `picture` at `LOGO`,
/// or none.
fn logo_of(picture: Option<&[u8]>) -> BTreeMap<String, String> {
    picture.map_or_else(BTreeMap::new, |bytes| {
        BTreeMap::from([(LOGO.to_owned(), file::sha256(bytes))])
    })
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

/// Runs the actors of `scenario` in `order`, each entry the index of the
/// actor that takes its next step, and asserts what each step must leave and
/// what must hold once every actor finishes. A claim that waits runs again
/// after the deletion's last step, followed by the steps its actor would have
/// taken meanwhile.
async fn run(store: &MongoStore, scenario: &Scenario, order: &[usize]) {
    let told = told(scenario, order);
    let mut runs: Vec<Run> = scenario.actors.iter().map(|_| Run::default()).collect();
    let mut turns: VecDeque<usize> = order.iter().copied().collect();
    let mut deferred = Vec::new();
    while let Some(actor) = turns.pop_front() {
        let current = &mut runs[actor];
        if current.waiting {
            deferred.push(actor);
            continue;
        }
        if current.stopped {
            continue;
        }
        let step = scenario.actors[actor].steps[current.next];
        if take(store, step, current).await {
            current.next += 1;
        } else if matches!(step, Step::Claim(..)) {
            current.waiting = true;
        } else {
            current.stopped = true;
        }
        assert_readable(&Holdings::of(store).await, &told);
        if matches!(step, Step::RemoveRecord) {
            let waiting = runs
                .iter_mut()
                .enumerate()
                .filter(|(_, other)| other.waiting);
            for (index, blocked) in waiting {
                let retried = scenario.actors[index].steps[blocked.next];
                assert!(
                    take(store, retried, blocked).await,
                    "{told}: a claim still waits once the deletion finished"
                );
                blocked.waiting = false;
                blocked.next += 1;
                assert_readable(&Holdings::of(store).await, &told);
            }
            for owed in mem::take(&mut deferred).into_iter().rev() {
                turns.push_front(owed);
            }
        }
    }
    assert!(
        runs.iter().all(|done| !done.waiting),
        "{told}: a claim still waits after the last step"
    );
    assert_finished(store, scenario, &told).await;
}

/// Puts `bundle` at `at` in `store`, replacing any document there.
async fn save(store: &MongoStore, at: &str, bundle: &BundledDocument) {
    let options = WriteOptions::default();
    put_bundle(store, &path(at), bundle, &options, &Precondition::None)
        .await
        .unwrap();
}

/// The four steps of a save of the document at `document` embedding
/// `picture` at `LOGO`.
const fn saving(document: &'static str, picture: &'static [u8]) -> [Step; 4] {
    [
        Step::Claim(document, picture),
        Step::Upload(document, picture),
        Step::Save(document, picture),
        Step::Release(document, Some(picture)),
    ]
}

/// A mapping of the one key `a`, to `value`.
fn single(value: Value) -> Mapping {
    Mapping::from([("a".to_owned(), value)])
}

/// Takes `step` for the actor whose run is `at`: `false` when its claim
/// waits for a deletion, or its mark finds nothing to mark.
async fn take(store: &MongoStore, step: Step, at: &mut Run) -> bool {
    match step {
        Step::Claim(document, picture) => {
            let saved = path(document);
            let files = logo_files(document, picture);
            for file in embedded(&saved, &parse(REPORT).document, &files).unwrap() {
                if !store.try_claim(&saved, &file).await.unwrap() {
                    return false;
                }
            }
        }
        Step::Mark(picture) => {
            if let Some(bytes) = picture {
                at.held = vec![file::sha256(bytes)];
            }
            let mut marked = Vec::new();
            for hash in mem::take(&mut at.held) {
                if store.mark(&hash).await.unwrap() {
                    marked.push(hash);
                }
            }
            at.held = marked;
            return !at.held.is_empty();
        }
        Step::Release(document, picture) => {
            at.held = store
                .release(&path(document), &logo_of(picture))
                .await
                .unwrap();
        }
        Step::RemoveDocument(document) => {
            store
                .remove_document(&path(document), &Precondition::None)
                .await
                .unwrap();
        }
        Step::RemoveObject => {
            for hash in &at.held {
                store.remove_object(hash).await.unwrap();
            }
        }
        Step::RemoveRecord => {
            for hash in &at.held {
                store.remove_record(hash).await.unwrap();
            }
        }
        Step::Save(document, picture) => {
            let saved = parse(REPORT).document;
            let files = logo_of(Some(picture));
            store
                .save(&path(document), &saved, &files, &Precondition::None)
                .await
                .unwrap();
        }
        Step::Upload(document, picture) => {
            let saved = path(document);
            let files = logo_files(document, picture);
            for file in embedded(&saved, &parse(REPORT).document, &files).unwrap() {
                store.upload(&file).await.unwrap();
            }
        }
    }
    true
}

/// `order` written as the step each entry gives, for a failing order's
/// message.
fn told(scenario: &Scenario, order: &[usize]) -> String {
    let mut taken = vec![0_usize; scenario.actors.len()];
    let mut steps = Vec::with_capacity(order.len());
    for &actor in order {
        let of = &scenario.actors[actor];
        steps.push(format!("{} {}", of.name, of.steps[taken[actor]].name()));
        taken[actor] += 1;
    }
    steps.join(", ")
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
async fn two_embeds_of_one_target_in_two_forms_give_each_file_the_alt_text_of_its_own_embed() {
    in_database("archie_test_same_target", async |store| {
        let note = b"# Logo\n";
        let files = BTreeMap::from([
            ("logo".to_owned(), PNG.to_vec()),
            ("logo.md".to_owned(), note.to_vec()),
        ]);
        let document = parse("![Picture](logo)\n\n![[logo|Note]]\n").document;
        let bundle = BundledDocument::new(document, files, Missing::Refuse).unwrap();
        save(&store, "notes/report.md", &bundle).await;
        assert_eq!(description_of(&store, PNG).await, "Picture");
        assert_eq!(description_of(&store, note).await, "Note");
    })
    .await;
}

#[tokio::test]
async fn an_embed_with_no_alt_gives_an_empty_description_and_a_note_s_own_embed_describes_its_file()
{
    in_database("archie_test_note_alt", async |store| {
        let chart = b"chart";
        let files = BTreeMap::from([
            ("a.png".to_owned(), PNG.to_vec()),
            ("b.png".to_owned(), OTHER_PNG.to_vec()),
            ("media/chart.png".to_owned(), chart.to_vec()),
            ("parts/intro.md".to_owned(), INTRO.as_bytes().to_vec()),
        ]);
        let document = parse("![](a.png)\n\n![[b.png]]\n\n![[parts/intro]]\n").document;
        let bundle = BundledDocument::new(document, files, Missing::Refuse).unwrap();
        save(&store, "notes/report.md", &bundle).await;
        assert_eq!(description_of(&store, PNG).await, "");
        assert_eq!(description_of(&store, OTHER_PNG).await, "");
        assert_eq!(description_of(&store, chart).await, "chart");
        assert_eq!(description_of(&store, INTRO.as_bytes()).await, "");
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
async fn a_save_that_replaces_a_picture_deletes_the_old_object_and_its_record() {
    in_database("archie_test_replace", async |store| {
        save(&store, "notes/report.md", &report_with(PNG)).await;
        save(&store, "notes/report.md", &report_with(OTHER_PNG)).await;
        let claims = [claim("notes/report.md", "notes/media/logo.png")];
        assert_eq!(claims_on(&store, OTHER_PNG).await, claims);
        assert_eq!(object_record(&store, PNG).await, None);
        let objects = store.files.list(None).await.unwrap();
        assert_eq!(objects, [object_of(OTHER_PNG)]);
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

        assert_eq!(store.try_claim(&report, picture).await, Ok(true));
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

        let released = store.release(&report, &hashes).await;
        assert_eq!(released, Ok(vec![file::sha256(PNG)]));
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
        assert_eq!(store.try_claim(&draft, &unsaved[0]).await, Ok(true));
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

#[tokio::test]
async fn a_plain_write_that_still_embeds_a_file_keeps_it() {
    in_database("archie_test_plain_write_keeps", async |store| {
        let report = path("notes/report.md");
        save(&store, "notes/report.md", &report_with(PNG)).await;
        store
            .write(&report, REPORT.as_bytes(), &Precondition::None)
            .await
            .unwrap();
        let back = get(&store, &report, Missing::Refuse).await.unwrap();
        assert_eq!(back, report_with(PNG));
        let documents = store.find(Document::new()).await.unwrap();
        let files = BTreeMap::from([("media/logo.png".to_owned(), file::sha256(PNG))]);
        assert_eq!(documents[0].files, files);
        let claims = [claim("notes/report.md", "notes/media/logo.png")];
        assert_eq!(claims_on(&store, PNG).await, claims);
        assert_eq!(store.files.list(None).await.unwrap(), [object_of(PNG)]);
    })
    .await;
}

#[tokio::test]
async fn a_plain_write_that_no_longer_embeds_a_file_releases_it_and_deletes_its_object() {
    in_database("archie_test_plain_write_releases", async |store| {
        let report = path("notes/report.md");
        save(&store, "notes/report.md", &report_with(PNG)).await;
        store
            .write(&report, b"# Report\n", &Precondition::None)
            .await
            .unwrap();
        let documents = store.find(Document::new()).await.unwrap();
        assert_eq!(documents[0].files, BTreeMap::new());
        assert_eq!(object_record(&store, PNG).await, None);
        assert_eq!(store.files.list(None).await, Ok(Vec::new()));
    })
    .await;
}

#[tokio::test]
async fn a_plain_write_of_a_new_document_names_no_file() {
    in_database("archie_test_plain_write_new", async |store| {
        let report = path("notes/report.md");
        store
            .write(&report, REPORT.as_bytes(), &Precondition::Absent)
            .await
            .unwrap();
        let documents = store.find(Document::new()).await.unwrap();
        assert_eq!(documents[0].files, BTreeMap::new());
        let records = store.objects.count_documents(Document::new()).await;
        assert_eq!(records.unwrap(), 0);
        assert_eq!(store.files.list(None).await, Ok(Vec::new()));
    })
    .await;
}

#[tokio::test]
async fn a_plain_write_that_still_embeds_a_note_keeps_the_note_and_the_files_it_embeds() {
    in_database("archie_test_plain_write_note", async |store| {
        let report = path("notes/report.md");
        let files = BTreeMap::from([
            ("media/chart.png".to_owned(), PNG.to_vec()),
            ("parts/intro.md".to_owned(), INTRO.as_bytes().to_vec()),
        ]);
        let document = parse(REPORT_WITH_INTRO).document;
        let bundle = BundledDocument::new(document, files, Missing::Refuse).unwrap();
        save(&store, "notes/report.md", &bundle).await;
        store
            .write(&report, REPORT_WITH_INTRO.as_bytes(), &Precondition::None)
            .await
            .unwrap();
        assert_eq!(get(&store, &report, Missing::Refuse).await.unwrap(), bundle);
        let chart = [claim("notes/report.md", "notes/media/chart.png")];
        assert_eq!(claims_on(&store, PNG).await, chart);
        let intro = [claim("notes/report.md", "notes/parts/intro.md")];
        assert_eq!(claims_on(&store, INTRO.as_bytes()).await, intro);
    })
    .await;
}

#[tokio::test]
async fn a_deletion_of_a_document_releases_its_claims_and_deletes_each_object_it_leaves_unembedded()
{
    in_database("archie_test_delete_document", async |store| {
        save(&store, "notes/report.md", &report_with(PNG)).await;
        save(&store, "drafts/report.md", &report_with(PNG)).await;
        let report = path("notes/report.md");
        let refused = store.delete(&report, &Precondition::Absent).await;
        assert_eq!(refused, Err(StoreError::Conflict(report.clone())));
        let both = [
            claim("notes/report.md", "notes/media/logo.png"),
            claim("drafts/report.md", "drafts/media/logo.png"),
        ];
        assert_eq!(claims_on(&store, PNG).await, both);

        store.delete(&report, &Precondition::None).await.unwrap();
        let drafts = [claim("drafts/report.md", "drafts/media/logo.png")];
        assert_eq!(claims_on(&store, PNG).await, drafts);
        assert_eq!(store.files.list(None).await.unwrap(), [object_of(PNG)]);

        let draft = path("drafts/report.md");
        store.delete(&draft, &Precondition::None).await.unwrap();
        assert_eq!(object_record(&store, PNG).await, None);
        assert_eq!(store.files.list(None).await, Ok(Vec::new()));
        assert_eq!(store.list(None).await, Ok(Vec::new()));
    })
    .await;
}

#[tokio::test]
async fn finishing_a_deletion_stopped_after_its_mark_deletes_the_object_and_the_record_and_a_save_claims_them_again()
 {
    in_database("archie_test_finish_after_mark", async |store| {
        finishes_a_stopped_deletion(&store, false).await;
    })
    .await;
}

#[tokio::test]
async fn finishing_a_deletion_stopped_after_its_object_was_removed_deletes_the_record_and_a_save_claims_them_again()
 {
    in_database("archie_test_finish_after_object", async |store| {
        finishes_a_stopped_deletion(&store, true).await;
    })
    .await;
}

#[tokio::test]
async fn finishing_deletions_where_no_deletion_holds_a_record_changes_nothing() {
    in_database("archie_test_finish_nothing", async |store| {
        begin(&store, Start::Embedded(PNG)).await;
        let record = object_record(&store, PNG).await;
        assert_eq!(store.finish_deletions().await, Ok(Vec::new()));
        assert_eq!(object_record(&store, PNG).await, record);
        let claims = [claim(REPORT_MD, "notes/media/logo.png")];
        assert_eq!(claims_on(&store, PNG).await, claims);
        assert_eq!(store.files.list(None).await.unwrap(), [object_of(PNG)]);
    })
    .await;
}

#[test]
fn a_release_that_fails_once_the_change_is_stored_says_the_change_is_stored() {
    let error = StoreError::Backend("server selection timed out".to_owned());
    let failed = stored_release_failed(&path(REPORT_MD), &error);
    let told = "notes/report.md: the change to the document is stored, and taking back the claims on the files it no longer embeds failed: server selection timed out";
    assert_eq!(failed, StoreError::Backend(told.to_owned()));
}

#[test]
fn a_deletion_that_stops_once_the_change_is_stored_says_so_and_names_the_command_that_finishes_it()
{
    let error = StoreError::Backend("server selection timed out".to_owned());
    let stopped = stopped_deletion(&path("files/0123"), &error);
    let told = "files/0123: the change to the document is stored, and the deletion of this object stopped: server selection timed out; `archie asset finish-deletions` finishes it";
    assert_eq!(stopped, StoreError::Backend(told.to_owned()));
}

#[test]
fn interleavings_give_each_order_that_keeps_each_actors_steps_in_sequence_once() {
    let mut orders = interleavings(&[2, 1]);
    orders.sort_unstable();
    assert_eq!(orders, [[0, 0, 1], [0, 1, 0], [1, 0, 0]]);
    assert_eq!(interleavings(&[4, 3]).len(), 35);
    assert_eq!(interleavings(&[4, 4, 3]).len(), 11_550);
}

#[test]
fn no_order_of_a_save_and_a_deletion_loses_an_object() {
    let orders = every_order("archie_test_orders_one_save", &ONE_SAVE);
    assert_eq!(orders, 35);
}

#[test]
fn no_order_of_two_saves_of_different_documents_and_a_deletion_loses_an_object() {
    let orders = every_order("archie_test_orders_two_saves", &TWO_SAVES);
    assert_eq!(orders, 11_550);
}

#[test]
fn every_order_of_a_save_that_replaces_a_picture_and_a_deletion_of_it_deletes_only_the_old_one() {
    let orders = every_order("archie_test_orders_replacement", &A_REPLACEMENT);
    assert_eq!(orders, 120);
}

#[test]
fn no_order_of_a_document_deletion_and_a_save_of_its_picture_loses_the_object() {
    let orders = every_order("archie_test_orders_document", &A_DOCUMENT_DELETION);
    assert_eq!(orders, 126);
}

#[test]
fn no_order_of_a_save_that_stops_after_its_claim_or_its_upload_and_a_deletion_loses_an_object() {
    let claimed = every_order("archie_test_orders_cut_claim", &A_SAVE_CUT_AFTER_ITS_CLAIM);
    assert_eq!(claimed, 4);
    let uploaded = every_order(
        "archie_test_orders_cut_upload",
        &A_SAVE_CUT_AFTER_ITS_UPLOAD,
    );
    assert_eq!(uploaded, 10);
}

#[test]
fn no_order_of_two_deletions_and_a_save_loses_an_object() {
    let orders = every_order("archie_test_orders_two_deletions", &TWO_DELETIONS);
    assert_eq!(orders, 4200);
}
