//! Whole documents. Each is a file under `fixtures/`, with the outline of
//! the tree it parses into beside it in a `.tree` file.

use super::check;

macro_rules! document {
    ($name:ident) => {
        #[test]
        fn $name() {
            check(
                include_str!(concat!("fixtures/", stringify!($name), ".md")),
                include_str!(concat!("fixtures/", stringify!($name), ".tree")),
            );
        }
    };
}

// TOML frontmatter.
document!(hugo_post);
document!(task);
document!(zola_page);

// YAML frontmatter.
document!(contact);
document!(docusaurus_page);
document!(issue_template);
document!(jekyll_post);
document!(meeting_notes);
document!(obsidian_note);
document!(pandoc_paper);
document!(recipe);
document!(slash_command);
document!(yaml_list_of_mappings);

// No frontmatter.
document!(api_reference);
document!(book_page);
document!(changelog);
document!(daily_note);
document!(essay);
document!(readme);
