//! `run` over `MemorySystem`, with byte buffers for the standard streams.

use core::iter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use archie_core::parser::ast::FrontmatterFormat;
use archie_core::parser::parse;
use archie_core::writer::{WriteOptions, write};
use clap::Parser as _;
use os_shim::System as _;
use os_shim::mock::MemorySystem;
use tokio::runtime::Builder;

use super::run;
use crate::cli::Cli;

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

const FORMS: [&str; 3] = ["markdown", "json", "rkyv"];

const JEKYLL_POST: &str = FIXTURES[10];

/// A server no process listens on, which the driver tries once and gives up
/// on at once: no server selection to wait for, no retry.
#[cfg(feature = "mongodb")]
const UNREACHABLE: &str = "mongodb://127.0.0.1:1/?loadBalanced=true&retryReads=false";

const ZOLA_PAGE: &str = FIXTURES[20];

/// What a run gave: its exit code and the two output streams.
struct Ran {
    code: ExitCode,
    stderr: String,
    stdout: Vec<u8>,
}

/// Runs `archie` and `arguments` over `system`, `stdin` on standard input.
fn archie(system: &MemorySystem, arguments: &[&str], stdin: &[u8]) -> Ran {
    let cli = Cli::try_parse_from(iter::once("archie").chain(arguments.iter().copied())).unwrap();
    let mut input = stdin;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let runtime = Builder::new_current_thread().enable_all().build().unwrap();
    let code = runtime.block_on(run(&cli, system, &mut input, &mut stdout, &mut stderr));
    Ran {
        code,
        stderr: String::from_utf8(stderr).unwrap(),
        stdout,
    }
}

/// archie's form of the markdown `source`, its frontmatter in `frontmatter`.
fn archies_form(source: &str, frontmatter: FrontmatterFormat) -> String {
    let mut options = WriteOptions::default();
    options.frontmatter = frontmatter;
    write(&parse(source).document, &options).unwrap()
}

/// Asserts `ran` failed with `code` and one line on standard error that
/// opens with `line`, and wrote nothing to standard output.
fn failed(ran: &Ran, code: u8, line: &str) {
    assert_eq!(ran.code, ExitCode::from(code), "{}", ran.stderr);
    assert!(ran.stderr.starts_with(line), "{}", ran.stderr);
    assert!(ran.stderr.ends_with('\n'), "{}", ran.stderr);
    assert_eq!(ran.stderr.lines().count(), 1, "{}", ran.stderr);
    assert_eq!(ran.stdout, b"");
}

fn read(system: &MemorySystem, path: &str) -> String {
    system.read_to_string(Path::new(path)).unwrap()
}

/// A system whose current folder is `/vault`, holding `a.md` with `source`.
fn vault(source: &str) -> MemorySystem {
    MemorySystem::new()
        .with_file("/vault/a.md", source.as_bytes())
        .unwrap()
        .with_current_dir("/vault")
        .unwrap()
}

#[test]
fn each_fixture_copied_in_each_form_is_archies_form_and_a_second_copy_changes_nothing() {
    for (index, source) in FIXTURES.iter().enumerate() {
        let archie_form = archies_form(source, FrontmatterFormat::Yaml);
        let system = vault(source);
        for form in FORMS {
            let first = format!("{form}/{index}.md");
            let second = format!("{form}/again/{index}.md");
            let got = archie(&system, &["get", "a.md", "-f", form], b"");
            assert_eq!(got.code, ExitCode::SUCCESS, "{form}: {}", got.stderr);
            let written = archie(&system, &["write", &first, "-f", form], &got.stdout);
            assert_eq!(
                written.code,
                ExitCode::SUCCESS,
                "{form}: {}",
                written.stderr
            );
            assert_eq!(written.stdout, b"");
            assert_eq!(written.stderr, "");
            let copied = read(&system, &format!("/vault/{first}"));
            assert_eq!(copied, archie_form, "{form}: {source}");
            let again = archie(&system, &["get", &first, "-f", form], b"");
            let rewritten = archie(&system, &["write", &second, "-f", form], &again.stdout);
            assert_eq!(
                rewritten.code,
                ExitCode::SUCCESS,
                "{form}: {}",
                rewritten.stderr
            );
            assert_eq!(read(&system, &format!("/vault/{second}")), copied);
        }
    }
}

#[test]
fn a_get_with_toml_frontmatter_of_a_yaml_file_writes_toml_and_yaml_when_left_out() {
    assert!(JEKYLL_POST.starts_with("---\n"));
    let system = vault(JEKYLL_POST);
    let toml = archie(&system, &["get", "a.md", "--frontmatter", "toml"], b"");
    assert_eq!(toml.code, ExitCode::SUCCESS, "{}", toml.stderr);
    assert!(toml.stdout.starts_with(b"+++\n"));
    let toml_form = archies_form(JEKYLL_POST, FrontmatterFormat::Toml);
    assert_eq!(toml.stdout, toml_form.as_bytes());
    assert!(ZOLA_PAGE.starts_with("+++\n"));
    let zola = vault(ZOLA_PAGE);
    let yaml = archie(&zola, &["get", "a.md"], b"");
    assert!(yaml.stdout.starts_with(b"---\n"));
    let yaml_form = archies_form(ZOLA_PAGE, FrontmatterFormat::Yaml);
    assert_eq!(yaml.stdout, yaml_form.as_bytes());
}

#[test]
fn a_write_with_toml_frontmatter_stores_toml_and_yaml_when_left_out() {
    let system = MemorySystem::new();
    let toml = archie(
        &system,
        &["write", "/vault/toml.md", "--frontmatter", "toml"],
        JEKYLL_POST.as_bytes(),
    );
    assert_eq!(toml.code, ExitCode::SUCCESS, "{}", toml.stderr);
    let toml_file = read(&system, "/vault/toml.md");
    assert!(toml_file.starts_with("+++\n"));
    assert_eq!(
        toml_file,
        archies_form(JEKYLL_POST, FrontmatterFormat::Toml)
    );
    let yaml = archie(&system, &["write", "/vault/yaml.md"], ZOLA_PAGE.as_bytes());
    assert_eq!(yaml.code, ExitCode::SUCCESS, "{}", yaml.stderr);
    let yaml_file = read(&system, "/vault/yaml.md");
    assert!(yaml_file.starts_with("---\n"));
}

#[test]
fn output_and_input_name_files_of_the_machine() {
    let source = FIXTURES[7];
    let system = vault(source);
    let streamed = archie(&system, &["get", "a.md", "-f", "json"], b"");
    let got = archie(
        &system,
        &["get", "a.md", "-f", "json", "-o", "out/a.json"],
        b"",
    );
    assert_eq!(got.code, ExitCode::SUCCESS, "{}", got.stderr);
    assert_eq!(got.stdout, b"");
    assert_eq!(got.stderr, "");
    assert_eq!(
        read(&system, "/vault/out/a.json").as_bytes(),
        streamed.stdout
    );
    let written = archie(
        &system,
        &["write", "b.md", "-f", "json", "-i", "out/a.json"],
        b"",
    );
    assert_eq!(written.code, ExitCode::SUCCESS, "{}", written.stderr);
    assert_eq!(
        read(&system, "/vault/b.md"),
        archies_form(source, FrontmatterFormat::Yaml)
    );
}

#[test]
fn a_failing_get_with_output_leaves_no_file_and_no_partial_file() {
    let system = vault("# A\n");
    let ran = archie(&system, &["get", "missing.md", "-o", "out.json"], b"");
    failed(&ran, 6, "archie: missing.md: no file at this path");
    let folder = system.read_dir(Path::new("/vault")).unwrap();
    assert_eq!(folder, [PathBuf::from("/vault/a.md")]);
}

#[test]
fn a_missing_path_exits_6_with_one_line_naming_the_path() {
    let system = vault("# A\n");
    let ran = archie(&system, &["get", "missing.md"], b"");
    failed(&ran, 6, "archie: missing.md: no file at this path");
    assert_eq!(ran.stderr, "archie: missing.md: no file at this path\n");
    let input = archie(&system, &["write", "b.md", "-i", "missing.json"], b"");
    failed(&input, 6, "archie: missing.json: no file at this path");
    assert!(!system.exists(Path::new("/vault/b.md")).unwrap());
}

#[test]
fn text_that_is_not_json_exits_1_naming_the_path_and_stores_nothing() {
    let system = vault("# A\n");
    let ran = archie(&system, &["write", "b.md", "-f", "json"], b"nope\n");
    failed(&ran, 1, "archie: b.md: ");
    assert!(!system.exists(Path::new("/vault/b.md")).unwrap());
}

#[test]
fn bytes_that_are_not_utf8_exit_1_naming_the_path_and_the_offset() {
    let system = MemorySystem::new().with_file("/a.md", b"a\xffb").unwrap();
    let ran = archie(&system, &["get", "a.md"], b"");
    failed(&ran, 1, "archie: a.md: not UTF-8 at byte 1");
}

#[test]
fn a_path_that_names_no_file_exits_2() {
    let system = vault("# A\n");
    let ran = archie(&system, &["get", "../.."], b"");
    failed(&ran, 2, "archie: ../..: ");
}

#[test]
fn a_minio_store_exits_3_in_this_build() {
    let system = vault("# A\n");
    let minio = archie(
        &system,
        &["write", "b.md", "-s", "minio:notes/drafts"],
        b"# B\n",
    );
    failed(
        &minio,
        3,
        "archie: minio:notes/drafts: this archie is built without the MinIO store",
    );
    assert!(!system.exists(Path::new("/vault/b.md")).unwrap());
}

#[cfg(not(feature = "mongodb"))]
#[test]
fn a_mongodb_store_exits_3_in_a_build_without_it() {
    let system = vault("# A\n");
    let mongodb = archie(&system, &["get", "a.md", "-s", "mongodb:notes/files"], b"");
    failed(
        &mongodb,
        3,
        "archie: mongodb:notes/files: this archie is built without the MongoDB store",
    );
}

#[cfg(feature = "mongodb")]
#[test]
fn a_mongodb_store_with_no_connection_string_exits_3_with_one_line_naming_the_variable() {
    let system = vault("# A\n");
    let line = "archie: ARCHIE_MONGODB_URI is not set: it holds the MongoDB connection string\n";
    let got = archie(&system, &["get", "a.md", "-s", "mongodb:notes/files"], b"");
    failed(&got, 3, line);
    let written = archie(
        &system,
        &["write", "b.md", "--store", "mongodb:notes/files"],
        b"# B\n",
    );
    failed(&written, 3, line);
    assert!(!system.exists(Path::new("/vault/b.md")).unwrap());
}

#[cfg(feature = "mongodb")]
#[test]
fn a_mongodb_server_that_cannot_be_reached_exits_4_naming_the_store() {
    let system = vault("# A\n")
        .with_env("ARCHIE_MONGODB_URI", UNREACHABLE)
        .unwrap();
    let got = archie(&system, &["get", "a.md", "-s", "mongodb:notes/files"], b"");
    failed(&got, 4, "archie: mongodb:notes/files: ");
    let written = archie(
        &system,
        &["write", "b.md", "-s", "mongodb:notes/files"],
        b"# B\n",
    );
    failed(&written, 4, "archie: mongodb:notes/files: ");
    assert!(!system.exists(Path::new("/vault/b.md")).unwrap());
}

#[cfg(feature = "mongodb")]
#[test]
fn with_a_mongodb_store_the_path_is_used_as_written_and_checked_before_connecting() {
    let system = vault("# A\n")
        .with_env("ARCHIE_MONGODB_URI", UNREACHABLE)
        .unwrap();
    for path in ["/notes/a.md", "./a.md", "notes/../a.md"] {
        let ran = archie(&system, &["get", path, "-s", "mongodb:notes/files"], b"");
        failed(
            &ran,
            2,
            &format!("archie: {path}: `{path}` is not a store path"),
        );
    }
}

#[test]
fn a_standard_output_that_fails_exits_4() {
    let system = vault("# A\n");
    let cli = Cli::try_parse_from(["archie", "get", "a.md"]).unwrap();
    let mut full: &mut [u8] = &mut [];
    let mut stderr = Vec::new();
    let runtime = Builder::new_current_thread().build().unwrap();
    let code = runtime.block_on(run(&cli, &system, &mut &b""[..], &mut full, &mut stderr));
    assert_eq!(code, ExitCode::from(4));
    let line = String::from_utf8(stderr).unwrap();
    assert!(line.starts_with("archie: standard output: "), "{line}");
    assert_eq!(line.lines().count(), 1, "{line}");
}

#[test]
fn a_relative_path_from_a_subfolder_names_the_file_the_shell_names() {
    let source = FIXTURES[17];
    let system = MemorySystem::new()
        .with_file("/vault/x/a.md", source.as_bytes())
        .unwrap()
        .with_dir("/vault/y")
        .unwrap()
        .with_current_dir("/vault/y")
        .unwrap();
    let expected = archies_form(source, FrontmatterFormat::Yaml);
    for path in [
        "../x/a.md",
        "./../x/./a.md",
        "/vault/x/a.md",
        "../../vault/x/a.md",
    ] {
        let ran = archie(&system, &["get", path], b"");
        assert_eq!(ran.code, ExitCode::SUCCESS, "{path}: {}", ran.stderr);
        assert_eq!(ran.stdout, expected.as_bytes(), "{path}");
    }
    let written = archie(&system, &["write", "../x/b.md"], source.as_bytes());
    assert_eq!(written.code, ExitCode::SUCCESS, "{}", written.stderr);
    assert_eq!(read(&system, "/vault/x/b.md"), expected);
}

#[test]
fn a_write_replaces_the_file_at_the_path() {
    let system = vault("# Old\n");
    let ran = archie(&system, &["write", "a.md"], b"# New\n");
    assert_eq!(ran.code, ExitCode::SUCCESS, "{}", ran.stderr);
    assert_eq!(read(&system, "/vault/a.md"), "# New\n");
}
