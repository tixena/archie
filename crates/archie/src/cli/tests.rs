use archie_core::forms::Form;
use clap::error::ErrorKind;
use clap::{CommandFactory as _, Parser as _};

use super::{Cli, Command, FormatArg, StoreArg, SyntaxArg};

#[test]
fn the_command_line_is_well_formed() {
    Cli::command().debug_assert();
}

#[test]
fn markdown_and_yaml_are_the_defaults_no_store_is_the_file_system_and_no_flag_is_set() {
    let get = Cli::try_parse_from(["archie", "get", "a.md"]).unwrap();
    assert!(
        matches!(
            &get.command,
            Command::Get {
                allow_missing: false,
                embed: false,
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
                allow_missing: false,
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
        "--allow-missing",
    ])
    .unwrap();
    assert!(
        matches!(
            &get.command,
            Command::Get {
                allow_missing: true,
                embed: false,
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
        "--allow-missing",
    ])
    .unwrap();
    assert!(
        matches!(
            &write.command,
            Command::Write {
                allow_missing: true,
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
fn each_of_the_four_forms_parses_on_get_and_write_and_names_its_form() {
    let forms = [
        ("binary", Form::Binary),
        ("json", Form::Json),
        ("markdown", Form::Markdown),
        ("zip", Form::Zip),
    ];
    for (text, form) in forms {
        for command in ["get", "write"] {
            let parsed = Cli::try_parse_from(["archie", command, "a.md", "-f", text]).unwrap();
            let (Command::Get {
                allow_missing: _,
                embed: _,
                format,
                frontmatter: _,
                output: _,
                path: _,
                store: _,
            }
            | Command::Write {
                allow_missing: _,
                format,
                frontmatter: _,
                input: _,
                path: _,
                store: _,
            }) = parsed.command;
            assert_eq!(Form::from(format), form, "{command} {text}");
        }
    }
}

#[test]
fn embed_with_an_output_parses() {
    let get = Cli::try_parse_from(["archie", "get", "a.md", "-o", "out/a.md", "--embed"]).unwrap();
    assert!(
        matches!(
            &get.command,
            Command::Get {
                allow_missing: false,
                embed: true,
                format: FormatArg::Markdown,
                frontmatter: SyntaxArg::Yaml,
                output: Some(output),
                path: _,
                store: None,
            } if output.as_os_str() == "out/a.md"
        ),
        "{get:?}"
    );
}

#[test]
fn embed_without_an_output_is_a_usage_error_with_code_2() {
    for arguments in [
        ["archie", "get", "a.md", "--embed"].as_slice(),
        ["archie", "get", "a.md", "-f", "zip", "--embed"].as_slice(),
    ] {
        let error = Cli::try_parse_from(arguments).unwrap_err();
        assert_eq!(
            error.kind(),
            ErrorKind::MissingRequiredArgument,
            "{arguments:?}"
        );
        assert_eq!(error.exit_code(), 2_i32, "{arguments:?}");
    }
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
        ["archie", "write", "b.md", "-i", "b.md", "--embed"].as_slice(),
    ] {
        let error = Cli::try_parse_from(arguments).unwrap_err();
        assert_eq!(error.exit_code(), 2_i32, "{arguments:?}");
    }
}
