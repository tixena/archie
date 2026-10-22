use clap::error::ErrorKind;
use clap::{CommandFactory as _, Parser as _};

use super::{Cli, Command, FormatArg, StoreArg, SyntaxArg};

#[test]
fn the_command_line_is_well_formed() {
    Cli::command().debug_assert();
}

#[test]
fn markdown_and_yaml_are_the_defaults_and_no_store_is_the_file_system() {
    let get = Cli::try_parse_from(["archie", "get", "a.md"]).unwrap();
    assert!(
        matches!(
            &get.command,
            Command::Get {
                format: FormatArg::Markdown,
                frontmatter: SyntaxArg::Yaml,
                output: None,
                path,
                store: None,
            } if path.as_os_str() == "a.md"
        ),
        "{get:?}"
    );
    let write = Cli::try_parse_from(["archie", "write", "b.md"]).unwrap();
    assert!(
        matches!(
            &write.command,
            Command::Write {
                format: FormatArg::Markdown,
                frontmatter: SyntaxArg::Yaml,
                input: None,
                path,
                store: None,
            } if path.as_os_str() == "b.md"
        ),
        "{write:?}"
    );
}

#[test]
fn the_short_and_long_options_parse() {
    let get = Cli::try_parse_from([
        "archie",
        "get",
        "a.md",
        "-f",
        "binary",
        "--frontmatter",
        "toml",
        "-o",
        "a.bin",
        "-s",
        "minio:notes",
    ])
    .unwrap();
    assert!(
        matches!(
            &get.command,
            Command::Get {
                format: FormatArg::Binary,
                frontmatter: SyntaxArg::Toml,
                output: Some(output),
                path: _,
                store: Some(StoreArg::Minio { bucket, prefix: None }),
            } if output.as_os_str() == "a.bin" && bucket == "notes"
        ),
        "{get:?}"
    );
    let write = Cli::try_parse_from([
        "archie",
        "write",
        "b.md",
        "--format",
        "json",
        "--input",
        "a.json",
        "--store",
        "mongodb:notes/files",
    ])
    .unwrap();
    assert!(
        matches!(
            &write.command,
            Command::Write {
                format: FormatArg::Json,
                frontmatter: SyntaxArg::Yaml,
                input: Some(input),
                path: _,
                store: Some(StoreArg::Mongodb { collection, database }),
            } if input.as_os_str() == "a.json" && collection == "files" && database == "notes"
        ),
        "{write:?}"
    );
}

#[test]
fn each_store_value_parses_and_displays_as_it_was_written() {
    let stores = [
        (
            "minio:notes",
            StoreArg::Minio {
                bucket: "notes".to_owned(),
                prefix: None,
            },
        ),
        (
            "minio:notes/drafts/2026",
            StoreArg::Minio {
                bucket: "notes".to_owned(),
                prefix: Some("drafts/2026".to_owned()),
            },
        ),
        (
            "mongodb:notes/files",
            StoreArg::Mongodb {
                collection: "files".to_owned(),
                database: "notes".to_owned(),
            },
        ),
    ];
    for (text, expected) in stores {
        let store = text.parse::<StoreArg>().unwrap();
        assert_eq!(store, expected);
        assert_eq!(store.to_string(), text);
    }
}

#[test]
fn a_store_of_another_shape_is_a_usage_error_with_code_2() {
    for text in [
        "",
        "notes",
        "minio:",
        "minio:/drafts",
        "minio:notes/",
        "mongodb:",
        "mongodb:notes",
        "mongodb:/files",
        "mongodb:notes/",
        "s3:notes",
    ] {
        let error = Cli::try_parse_from(["archie", "get", "a.md", "-s", text]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ValueValidation, "{text}");
        assert_eq!(error.exit_code(), 2_i32, "{text}");
    }
}

#[test]
fn an_unknown_option_a_missing_path_and_an_unknown_value_are_usage_errors_with_code_2() {
    for arguments in [
        ["archie", "get", "a.md", "--nope"].as_slice(),
        ["archie", "get"].as_slice(),
        ["archie", "write"].as_slice(),
        ["archie", "get", "a.md", "-f", "bincode"].as_slice(),
        ["archie", "write", "b.md", "--frontmatter", "json"].as_slice(),
        ["archie", "write", "b.md", "-o", "b.json"].as_slice(),
        ["archie", "get", "a.md", "-i", "a.json"].as_slice(),
    ] {
        let error = Cli::try_parse_from(arguments).unwrap_err();
        assert_eq!(error.exit_code(), 2_i32, "{arguments:?}");
    }
}
