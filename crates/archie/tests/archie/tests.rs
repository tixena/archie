#[cfg(any(feature = "minio", feature = "mongodb"))]
use std::ffi::OsStr;
use std::fs::{self, File};
#[cfg(any(feature = "minio", feature = "mongodb"))]
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};

use archie_core::parser::ast::FrontmatterFormat;
use archie_core::parser::parse;
use archie_core::writer::{WriteOptions, write};
use assert_cmd::Command;
use assert_cmd::assert::{Assert, OutputAssertExt as _};
use assert_cmd::cargo::cargo_bin;
use predicates::ord::eq;
use predicates::str::{is_empty, starts_with};
use tempfile::TempDir;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../archie-core/src/parser/tests/fixtures"
);

const FORMS: [&str; 3] = ["markdown", "json", "rkyv"];

/// The four variables a `MinIO` store reads: a server no process listens on,
/// whose connection is refused at once, and keys no output may show.
#[cfg(feature = "minio")]
const MINIO_VARIABLES: [(&str, &str); 4] = [
    ("ARCHIE_MINIO_ENDPOINT", "http://127.0.0.1:1"),
    ("ARCHIE_MINIO_REGION", "us-east-1"),
    ("AWS_ACCESS_KEY_ID", "archie-binary-access"),
    ("AWS_SECRET_ACCESS_KEY", "archie-binary-secret"),
];

/// A server no process listens on, which the driver tries once and gives up
/// on at once: no server selection to wait for, no retry.
#[cfg(feature = "mongodb")]
const UNREACHABLE: &str = "mongodb://127.0.0.1:1/?loadBalanced=true&retryReads=false";

/// Runs the binary in `folder` with `arguments`, `stdin` on standard input.
fn archie(folder: &Path, arguments: &[&str], stdin: &[u8]) -> Assert {
    Command::cargo_bin("archie")
        .unwrap()
        .current_dir(folder)
        .args(arguments)
        .write_stdin(stdin)
        .assert()
}

/// Runs the binary in `folder` with `arguments`, `ARCHIE_MONGODB_URI` set to
/// `uri` or removed when it is `None`.
#[cfg(feature = "mongodb")]
fn archie_with_uri(folder: &Path, arguments: &[&str], uri: Option<&OsStr>) -> Assert {
    let mut command = Command::cargo_bin("archie").unwrap();
    command.current_dir(folder).args(arguments);
    if let Some(value) = uri {
        command.env("ARCHIE_MONGODB_URI", value);
    } else {
        command.env_remove("ARCHIE_MONGODB_URI");
    }
    command.write_stdin(b"# B\n".as_slice()).assert()
}

/// Runs the binary in `folder` with `arguments`, `# B` on standard input,
/// and of the variables of `MINIO_VARIABLES` only those in `variables`.
#[cfg(feature = "minio")]
fn archie_with_minio(folder: &Path, arguments: &[&str], variables: &[(&str, &OsStr)]) -> Assert {
    let mut command = Command::cargo_bin("archie").unwrap();
    command.current_dir(folder).args(arguments);
    for (name, _) in MINIO_VARIABLES {
        command.env_remove(name);
    }
    command.envs(variables.iter().copied());
    command.write_stdin(b"# B\n".as_slice()).assert()
}

/// archie's form of the markdown `source`, its frontmatter in `frontmatter`.
fn archies_form(source: &str, frontmatter: FrontmatterFormat) -> String {
    let mut options = WriteOptions::default();
    options.frontmatter = frontmatter;
    write(&parse(source).document, &options).unwrap()
}

/// Asserts the run failed with `code` and one line on standard error that
/// opens with `line`, and wrote nothing to standard output.
fn failed(ran: Assert, code: i32, line: &str) {
    let stderr = String::from_utf8(ran.get_output().stderr.clone()).unwrap();
    ran.code(code).stdout(is_empty()).stderr(starts_with(line));
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.ends_with('\n'), "{stderr}");
}

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

/// Asserts as `failed` does, and that standard error shows neither key of
/// `MINIO_VARIABLES`.
#[cfg(feature = "minio")]
fn failed_hiding_the_keys(ran: Assert, code: i32, line: &str) {
    let stderr = String::from_utf8(ran.get_output().stderr.clone()).unwrap();
    for (_, key) in &MINIO_VARIABLES[2..] {
        assert!(!stderr.contains(key), "{stderr}");
    }
    failed(ran, code, line);
}

/// A new folder under the target folder.
fn folder() -> TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap()
}

/// The names in `folder`, sorted.
fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort_unstable();
    names
}

/// Takes the write permission off `folder`.
fn read_only(folder: &Path) {
    let mut permissions = fs::metadata(folder).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(folder, permissions).unwrap();
}

fn stdout(ran: &Assert) -> Vec<u8> {
    ran.get_output().stdout.clone()
}

#[test]
fn the_copy_of_each_fixture_in_each_form_is_archies_form_and_a_second_copy_changes_nothing() {
    let folder = folder();
    let root = folder.path();
    for (name, source) in fixtures() {
        fs::write(root.join(&name), &source).unwrap();
        let archie_form = archies_form(&source, FrontmatterFormat::Yaml);
        for form in FORMS {
            let first = format!("{form}/{name}");
            let second = format!("{form}/again/{name}");
            let got = archie(root, &["get", &name, "-f", form], b"").success();
            archie(root, &["write", &first, "-f", form], &stdout(&got))
                .success()
                .stdout(is_empty())
                .stderr(is_empty());
            let copied = fs::read_to_string(root.join(&first)).unwrap();
            assert_eq!(copied, archie_form, "{form}: {name}");
            let again = archie(root, &["get", &first, "-f", form], b"").success();
            archie(root, &["write", &second, "-f", form], &stdout(&again)).success();
            assert_eq!(fs::read_to_string(root.join(&second)).unwrap(), copied);
        }
    }
}

#[test]
fn a_pipe_between_two_runs_copies_the_file_in_each_form() {
    let folder = folder();
    let root = folder.path();
    let source = fs::read_to_string(Path::new(FIXTURES).join("hugo_post.md")).unwrap();
    fs::write(root.join("a.md"), &source).unwrap();
    for form in FORMS {
        let target = format!("{form}.md");
        let mut get = Process::new(cargo_bin("archie"))
            .current_dir(root)
            .args(["get", "a.md", "-f", form])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let pipe = get.stdout.take().unwrap();
        let written = Process::new(cargo_bin("archie"))
            .current_dir(root)
            .args(["write", &target, "-f", form])
            .stdin(pipe)
            .status()
            .unwrap();
        assert!(get.wait().unwrap().success());
        assert!(written.success());
        let copied = fs::read_to_string(root.join(&target)).unwrap();
        assert_eq!(
            copied,
            archies_form(&source, FrontmatterFormat::Yaml),
            "{form}"
        );
    }
}

#[test]
fn toml_frontmatter_on_get_and_on_write_and_yaml_when_left_out() {
    let folder = folder();
    let root = folder.path();
    let jekyll = fs::read_to_string(Path::new(FIXTURES).join("jekyll_post.md")).unwrap();
    let zola = fs::read_to_string(Path::new(FIXTURES).join("zola_page.md")).unwrap();
    assert!(jekyll.starts_with("---\n") && zola.starts_with("+++\n"));
    fs::write(root.join("jekyll.md"), &jekyll).unwrap();
    fs::write(root.join("zola.md"), &zola).unwrap();
    let toml_form = archies_form(&jekyll, FrontmatterFormat::Toml);
    archie(root, &["get", "jekyll.md", "--frontmatter", "toml"], b"")
        .success()
        .stdout(eq(toml_form.as_bytes()));
    archie(root, &["get", "zola.md"], b"")
        .success()
        .stdout(starts_with("---\n"));
    archie(
        root,
        &["write", "toml.md", "--frontmatter", "toml"],
        jekyll.as_bytes(),
    )
    .success();
    assert_eq!(fs::read_to_string(root.join("toml.md")).unwrap(), toml_form);
    archie(root, &["write", "yaml.md"], zola.as_bytes()).success();
    let yaml_file = fs::read_to_string(root.join("yaml.md")).unwrap();
    assert!(yaml_file.starts_with("---\n"), "{yaml_file}");
}

#[test]
fn output_and_input_name_files_of_the_machine() {
    let folder = folder();
    let root = folder.path();
    let source = fs::read_to_string(Path::new(FIXTURES).join("recipe.md")).unwrap();
    fs::write(root.join("a.md"), &source).unwrap();
    let streamed = archie(root, &["get", "a.md", "-f", "json"], b"").success();
    archie(
        root,
        &["get", "a.md", "--format", "json", "--output", "out.json"],
        b"",
    )
    .success()
    .stdout(is_empty())
    .stderr(is_empty());
    assert_eq!(fs::read(root.join("out.json")).unwrap(), stdout(&streamed));
    archie(
        root,
        &["write", "b.md", "-f", "json", "-i", "out.json"],
        b"",
    )
    .success();
    let copied = fs::read_to_string(root.join("b.md")).unwrap();
    assert_eq!(copied, archies_form(&source, FrontmatterFormat::Yaml));
}

#[test]
fn a_failing_get_with_output_leaves_no_file_and_no_partial_file() {
    let folder = folder();
    let root = folder.path();
    fs::write(root.join("a.md"), "# A\n").unwrap();
    let missing = archie(root, &["get", "missing.md", "-o", "out.md"], b"");
    failed(missing, 6, "archie: missing.md: no file at this path");
    assert_eq!(names(root), ["a.md"]);
    let locked = root.join("locked");
    fs::create_dir_all(&locked).unwrap();
    read_only(&locked);
    let refused = archie(root, &["get", "a.md", "-o", "locked/out.md"], b"");
    failed(refused, 4, "archie: locked/out.md: ");
    assert_eq!(names(&locked), Vec::<String>::new());
}

#[test]
fn a_missing_path_exits_6_with_one_line_naming_the_path() {
    let folder = folder();
    let ran = archie(folder.path(), &["get", "missing.md"], b"");
    ran.code(6_i32)
        .stdout(is_empty())
        .stderr(eq("archie: missing.md: no file at this path\n"));
}

#[test]
fn text_that_is_not_json_exits_1_and_stores_nothing() {
    let folder = folder();
    let root = folder.path();
    let ran = archie(root, &["write", "b.md", "-f", "json"], b"nope\n");
    failed(ran, 1, "archie: b.md: ");
    assert_eq!(names(root), Vec::<String>::new());
}

#[test]
fn an_unknown_option_and_a_missing_path_exit_2() {
    let folder = folder();
    let root = folder.path();
    archie(root, &["get", "a.md", "--nope"], b"")
        .code(2_i32)
        .stdout(is_empty());
    archie(root, &["get"], b"").code(2_i32).stdout(is_empty());
}

#[cfg(not(feature = "mongodb"))]
#[test]
fn a_mongodb_store_exits_3_in_a_build_without_it() {
    let folder = folder();
    let root = folder.path();
    fs::write(root.join("a.md"), "# A\n").unwrap();
    archie(
        root,
        &["get", "a.md", "--store", "mongodb:notes/files"],
        b"",
    )
    .code(3_i32)
    .stdout(is_empty())
    .stderr(eq(
        "archie: mongodb:notes/files: this archie is built without the MongoDB store\n",
    ));
}

#[cfg(feature = "mongodb")]
#[test]
fn a_mongodb_store_with_no_connection_string_exits_3_with_one_line_naming_the_variable() {
    let folder = folder();
    let root = folder.path();
    fs::write(root.join("a.md"), "# A\n").unwrap();
    let get = ["get", "a.md", "--store", "mongodb:notes/files"];
    archie_with_uri(root, &get, None)
        .code(3_i32)
        .stdout(is_empty())
        .stderr(eq(
            "archie: ARCHIE_MONGODB_URI is not set: it holds the MongoDB connection string\n",
        ));
    let not_utf8 = OsStr::from_bytes(b"mongodb://\xff");
    archie_with_uri(
        root,
        &["write", "b.md", "-s", "mongodb:notes/files"],
        Some(not_utf8),
    )
    .code(3_i32)
    .stdout(is_empty())
    .stderr(eq(
        "archie: ARCHIE_MONGODB_URI is not UTF-8: it holds the MongoDB connection string\n",
    ));
    assert_eq!(names(root), ["a.md"]);
}

#[cfg(feature = "mongodb")]
#[test]
fn a_mongodb_server_that_cannot_be_reached_exits_4_naming_the_store() {
    let folder = folder();
    let root = folder.path();
    let uri = Some(OsStr::new(UNREACHABLE));
    let get = archie_with_uri(root, &["get", "a.md", "-s", "mongodb:notes/files"], uri);
    failed(get, 4, "archie: mongodb:notes/files: ");
    let write = archie_with_uri(root, &["write", "b.md", "-s", "mongodb:notes/files"], uri);
    failed(write, 4, "archie: mongodb:notes/files: ");
    assert_eq!(names(root), Vec::<String>::new());
}

#[cfg(not(feature = "minio"))]
#[test]
fn a_minio_store_exits_3_in_a_build_without_it() {
    let folder = folder();
    let root = folder.path();
    archie(
        root,
        &["write", "b.md", "-s", "minio:notes/drafts"],
        b"# B\n",
    )
    .code(3_i32)
    .stdout(is_empty())
    .stderr(eq(
        "archie: minio:notes/drafts: this archie is built without the MinIO store\n",
    ));
    assert_eq!(names(root), Vec::<String>::new());
}

#[cfg(feature = "minio")]
#[test]
fn each_minio_variable_left_out_or_not_utf8_exits_3_with_one_line_naming_it() {
    let folder = folder();
    let root = folder.path();
    fs::write(root.join("a.md"), "# A\n").unwrap();
    let all: Vec<(&str, &OsStr)> = MINIO_VARIABLES
        .iter()
        .map(|(name, value)| (*name, OsStr::new(value)))
        .collect();
    let lines = [
        "archie: ARCHIE_MINIO_ENDPOINT is not set: it holds the MinIO server's address\n",
        "archie: ARCHIE_MINIO_REGION is not set: it holds the region the MinIO server expects\n",
        "archie: minio:notes/vault: `AWS_ACCESS_KEY_ID` is not set\n",
        "archie: minio:notes/vault: `AWS_SECRET_ACCESS_KEY` is not set\n",
    ];
    for (index, line) in lines.iter().enumerate() {
        let mut variables = all.clone();
        variables.remove(index);
        let get = ["get", "a.md", "--store", "minio:notes/vault"];
        failed_hiding_the_keys(archie_with_minio(root, &get, &variables), 3, line);
        let write = ["write", "b.md", "-s", "minio:notes/vault"];
        failed_hiding_the_keys(archie_with_minio(root, &write, &variables), 3, line);
    }
    let not_utf8 = OsStr::from_bytes(b"http://\xff");
    let mut endpoint = all.clone();
    endpoint[0] = ("ARCHIE_MINIO_ENDPOINT", not_utf8);
    failed_hiding_the_keys(
        archie_with_minio(root, &["get", "a.md", "-s", "minio:notes/vault"], &endpoint),
        3,
        "archie: ARCHIE_MINIO_ENDPOINT is not UTF-8: it holds the MinIO server's address\n",
    );
    let mut secret = all;
    secret[3] = ("AWS_SECRET_ACCESS_KEY", OsStr::from_bytes(b"\xff"));
    failed_hiding_the_keys(
        archie_with_minio(root, &["write", "b.md", "-s", "minio:notes/vault"], &secret),
        3,
        "archie: minio:notes/vault: `AWS_SECRET_ACCESS_KEY` is not Unicode\n",
    );
    assert_eq!(names(root), ["a.md"]);
}

#[cfg(feature = "minio")]
#[test]
fn a_minio_server_that_cannot_be_reached_exits_4_naming_the_path_and_no_key() {
    let folder = folder();
    let root = folder.path();
    let variables: Vec<(&str, &OsStr)> = MINIO_VARIABLES
        .iter()
        .map(|(name, value)| (*name, OsStr::new(value)))
        .collect();
    let get = archie_with_minio(root, &["get", "a.md", "-s", "minio:notes"], &variables);
    failed_hiding_the_keys(get, 4, "archie: a.md: ");
    let write = ["write", "b.md", "-s", "minio:notes/vault"];
    let written = archie_with_minio(root, &write, &variables);
    failed_hiding_the_keys(written, 4, "archie: b.md: ");
    assert_eq!(names(root), Vec::<String>::new());
}

#[test]
fn a_write_into_a_folder_that_is_not_writable_exits_4() {
    let folder = folder();
    let root = folder.path();
    let locked = root.join("locked");
    fs::create_dir_all(&locked).unwrap();
    read_only(&locked);
    let ran = archie(root, &["write", "locked/b.md"], b"# B\n");
    failed(ran, 4, "archie: locked/b.md: ");
    assert_eq!(names(&locked), Vec::<String>::new());
}

#[test]
fn a_standard_input_that_cannot_be_read_exits_4() {
    let folder = folder();
    let root = folder.path();
    let ran = Process::new(cargo_bin("archie"))
        .current_dir(root)
        .args(["write", "b.md"])
        .stdin(File::open(root).unwrap())
        .output()
        .unwrap()
        .assert();
    failed(ran, 4, "archie: standard input: ");
    assert_eq!(names(root), Vec::<String>::new());
}

#[test]
fn a_relative_path_from_a_subfolder_names_the_file_the_shell_names() {
    let folder = folder();
    let root = folder.path();
    let source = fs::read_to_string(Path::new(FIXTURES).join("task.md")).unwrap();
    fs::create_dir_all(root.join("x")).unwrap();
    fs::create_dir_all(root.join("y")).unwrap();
    fs::write(root.join("x/a.md"), &source).unwrap();
    let expected = archies_form(&source, FrontmatterFormat::Yaml);
    let subfolder: PathBuf = root.join("y");
    archie(&subfolder, &["get", "../x/a.md"], b"")
        .success()
        .stdout(eq(expected.as_bytes()));
    archie(&subfolder, &["write", "../x/b.md"], source.as_bytes()).success();
    assert_eq!(fs::read_to_string(root.join("x/b.md")).unwrap(), expected);
}

#[test]
fn help_and_version_exit_0() {
    let folder = folder();
    let root = folder.path();
    archie(root, &["--version"], b"")
        .success()
        .stdout(eq(concat!("archie ", env!("CARGO_PKG_VERSION"), "\n")));
    archie(root, &["--help"], b"")
        .success()
        .stdout(starts_with("Markdown documents as a typed tree\n"));
}
