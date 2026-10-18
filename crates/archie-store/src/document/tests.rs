//! `get` and `put` over `AnyStore::File`, each scenario on `MemorySystem` and
//! on `RealSystem` in a temporary folder.

use std::path::PathBuf;

use archie_core::forms::{DecodeError, Form, FormError, RKYV_MAGIC, RKYV_VERSION};
use archie_core::parser::ast::{FrontmatterFormat, RawDocument};
use archie_core::parser::{from_json, parse, to_json};
use archie_core::writer::{WriteOptions, write};
use os_shim::System as _;
use os_shim::mock::MemorySystem;
use os_shim::real::RealSystem;
use tokio::runtime::Builder;

use super::{DocumentError, get, put};
use crate::any::AnyStore;
use crate::file::FileStore;
use crate::{Precondition, Revision, Store as _, StoreError, StorePath};

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

const FORMS: [Form; 3] = [Form::Json, Form::Markdown, Form::Rkyv];

/// Runs `scenario` over a store on each system.
fn on_both<S>(scenario: &S)
where
    S: AsyncFn(&AnyStore<'_>),
{
    let runtime = Builder::new_current_thread().build().unwrap();
    let memory = MemorySystem::new();
    let in_memory = AnyStore::File(FileStore::new(PathBuf::from("/vault"), &memory));
    runtime.block_on(scenario(&in_memory));
    let real = RealSystem::new();
    let folder = real.create_temp_dir().unwrap();
    let on_disk = AnyStore::File(FileStore::new(folder.path().to_path_buf(), &real));
    runtime.block_on(scenario(&on_disk));
}

/// Gets the document at `from` in `form` and puts those bytes at `to`.
async fn copy(store: &AnyStore<'_>, from: &StorePath, to: &StorePath, form: Form) -> Revision {
    let options = WriteOptions::default();
    let bytes = get(store, from, form, &options).await.unwrap();
    put(store, to, form, &bytes, &options, &Precondition::None)
        .await
        .unwrap()
}

/// Compiles only for a value that can go to another thread.
const fn sent<T>(value: T) -> T
where
    T: Send,
{
    value
}

#[tokio::test]
async fn the_sample_copies_a_list_through_json() {
    let system = MemorySystem::new();
    let store = AnyStore::File(FileStore::new("/vault".into(), &system));
    let options = WriteOptions::default();
    let a = StorePath::new("a.md").unwrap();
    let b = StorePath::new("b.md").unwrap();
    put(
        &store,
        &a,
        Form::Markdown,
        b"* item\n",
        &options,
        &Precondition::None,
    )
    .await
    .unwrap();
    let json = get(&store, &a, Form::Json, &options).await.unwrap();
    put(&store, &b, Form::Json, &json, &options, &Precondition::None)
        .await
        .unwrap();
    assert_eq!(store.read(&b).await.unwrap().bytes, b"- item\n");
}

async fn each_fixture_copied(store: &AnyStore<'_>) {
    for (index, source) in FIXTURES.iter().enumerate() {
        let document = parse(source).document;
        let archie = write(&document, &WriteOptions::default()).unwrap();
        let kept: RawDocument = from_json(&to_json(&document).unwrap()).unwrap();
        let through_json = write(&kept, &WriteOptions::default()).unwrap();
        let file = StorePath::new(&format!("fixtures/{index}.md")).unwrap();
        store
            .write(&file, source.as_bytes(), &Precondition::None)
            .await
            .unwrap();
        for form in FORMS {
            let first = StorePath::new(&format!("{form:?}/{index}.md")).unwrap();
            let second = StorePath::new(&format!("{form:?}/again/{index}.md")).unwrap();
            let revision = copy(store, &file, &first, form).await;
            let copied = store.read(&first).await.unwrap().bytes;
            let expected = if form == Form::Json {
                &through_json
            } else {
                &archie
            };
            assert_eq!(copied, expected.as_bytes(), "{form:?}: {source}");
            assert_eq!(copy(store, &first, &second, form).await, revision);
            let again = store.read(&second).await.unwrap().bytes;
            assert_eq!(again, copied, "{form:?}: {source}");
        }
    }
}

#[test]
fn each_fixture_copied_in_each_form_is_archies_form_and_a_second_copy_changes_nothing() {
    on_both(&each_fixture_copied);
}

async fn not_utf8(store: &AnyStore<'_>) {
    let path = StorePath::new("notes/a.md").unwrap();
    store
        .write(&path, b"a\xffb", &Precondition::None)
        .await
        .unwrap();
    for form in FORMS {
        let error = get(store, &path, form, &WriteOptions::default())
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "notes/a.md: not UTF-8 at byte 1");
        assert!(
            matches!(
                &error,
                DocumentError::Form {
                    path: failed,
                    error: FormError::Decode(DecodeError::NotUtf8 { offset: 1 }),
                } if *failed == path
            ),
            "{error:?}"
        );
    }
}

#[test]
fn a_get_of_a_file_that_is_not_utf8_names_the_path_and_the_offset() {
    on_both(&not_utf8);
}

async fn put_not_utf8(store: &AnyStore<'_>) {
    let path = StorePath::new("notes/a.md").unwrap();
    let options = WriteOptions::default();
    let refused = put(
        store,
        &path,
        Form::Markdown,
        b"a\xffb",
        &options,
        &Precondition::None,
    )
    .await
    .unwrap_err();
    assert_eq!(refused.to_string(), "notes/a.md: not UTF-8 at byte 1");
    assert_eq!(store.list(None).await, Ok(Vec::new()));
}

#[test]
fn a_put_of_bytes_that_are_not_utf8_names_the_path_and_writes_nothing() {
    on_both(&put_not_utf8);
}

async fn absent_over_a_file(store: &AnyStore<'_>) {
    let path = StorePath::new("a.md").unwrap();
    store
        .write(&path, b"# A\n", &Precondition::None)
        .await
        .unwrap();
    let old = store.read(&path).await.unwrap();
    let refused = put(
        store,
        &path,
        Form::Markdown,
        b"# B\n",
        &WriteOptions::default(),
        &Precondition::Absent,
    )
    .await;
    assert!(
        matches!(
            &refused,
            Err(DocumentError::Store(StoreError::Conflict(conflict))) if *conflict == path
        ),
        "{refused:?}"
    );
    assert_eq!(store.read(&path).await.unwrap(), old);
}

#[test]
fn a_put_that_needs_no_file_over_a_file_is_a_conflict_and_changes_nothing() {
    on_both(&absent_over_a_file);
}

async fn toml_of_yaml(store: &AnyStore<'_>) {
    let source = include_str!("../../../archie-core/src/parser/tests/fixtures/jekyll_post.md");
    assert!(source.starts_with("---\n"));
    let path = StorePath::new("post.md").unwrap();
    store
        .write(&path, source.as_bytes(), &Precondition::None)
        .await
        .unwrap();
    let mut toml = WriteOptions::default();
    toml.frontmatter = FrontmatterFormat::Toml;
    let bytes = get(store, &path, Form::Markdown, &toml).await.unwrap();
    assert!(bytes.starts_with(b"+++\n"));
    let expected = write(&parse(source).document, &toml).unwrap();
    assert_eq!(bytes, expected.as_bytes());
}

#[test]
fn a_get_with_toml_options_of_a_yaml_file_gives_toml_frontmatter() {
    on_both(&toml_of_yaml);
}

async fn rkyv_copied(store: &AnyStore<'_>) {
    let options = WriteOptions::default();
    let path = StorePath::new("a.md").unwrap();
    store
        .write(&path, b"* item\n", &Precondition::None)
        .await
        .unwrap();
    let rkyv = get(store, &path, Form::Rkyv, &options).await.unwrap();
    assert_eq!(
        &rkyv[..8],
        [RKYV_MAGIC.as_slice(), &[RKYV_VERSION]].concat()
    );
    put(
        store,
        &path,
        Form::Rkyv,
        &rkyv,
        &options,
        &Precondition::None,
    )
    .await
    .unwrap();
    assert_eq!(store.read(&path).await.unwrap().bytes, b"- item\n");
}

#[test]
fn a_get_in_rkyv_puts_back_as_archies_form() {
    on_both(&rkyv_copied);
}

#[tokio::test]
async fn the_futures_of_get_and_put_over_any_store_can_be_sent_to_another_thread() {
    let system = MemorySystem::new();
    let store = AnyStore::File(FileStore::new(PathBuf::from("/vault"), &system));
    let options = WriteOptions::default();
    let path = StorePath::new("a.md").unwrap();
    let written = put(
        &store,
        &path,
        Form::Markdown,
        b"* item\n",
        &options,
        &Precondition::None,
    );
    sent(written).await.unwrap();
    let read = get(&store, &path, Form::Markdown, &options);
    assert_eq!(sent(read).await.unwrap(), b"- item\n");
}
