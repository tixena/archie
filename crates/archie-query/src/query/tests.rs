use crate::query::Query;

const OPEN_ISSUE: &str = "+++\ntype = \"issue\"\nbug = \"B-12\"\nstatus = \"open\"\ntags = [\"parser\", \"rule\"]\ndue = 2031-05-01\n+++\n";
const CLOSED_ISSUE: &str = "+++\ntype = \"issue\"\nbug = \"B-12\"\nstatus = \"closed\"\n+++\n";
const OPEN_ISSUES_OF_B12: &str =
    "fm.type == \"issue\" and fm.bug == \"B-12\" and fm.status != \"closed\"";

fn matches(source: &str, query: &str) -> bool {
    Query::parse(query).unwrap().matches_source(source).unwrap()
}

#[test]
fn open_issues_of_a_bug_match() {
    assert!(matches(OPEN_ISSUE, OPEN_ISSUES_OF_B12));
}

#[test]
fn a_closed_issue_does_not_match() {
    assert!(!matches(CLOSED_ISSUE, OPEN_ISSUES_OF_B12));
}

#[test]
fn a_missing_field_equals_nothing() {
    assert!(!matches(OPEN_ISSUE, "fm.owner == \"lucia\""));
    assert!(!matches(OPEN_ISSUE, "fm.owner < 3"));
    assert!(matches(OPEN_ISSUE, "fm.owner != \"lucia\""));
}

#[test]
fn has_finds_an_item_in_a_list() {
    assert!(matches(OPEN_ISSUE, "fm.tags has \"rule\""));
    assert!(!matches(OPEN_ISSUE, "fm.tags has \"design\""));
    assert!(matches(OPEN_ISSUE, "fm.bug has \"12\""));
}

#[test]
fn dates_compare_in_order() {
    assert!(matches(OPEN_ISSUE, "fm.due < 2031-06-01"));
    assert!(matches(OPEN_ISSUE, "fm.due >= 2031-05-01"));
    assert!(!matches(OPEN_ISSUE, "fm.due > 2031-05-01"));
    assert!(matches(OPEN_ISSUE, "fm.due < 2031-05-01T12:00:00Z"));
}

#[test]
fn a_yaml_date_string_compares_as_a_date() {
    let yaml = "---\ndue: \"2031-05-01\"\n---\n";
    assert!(matches(yaml, "fm.due < 2031-06-01"));
    assert!(matches(yaml, "fm.due == 2031-05-01"));
}

#[test]
fn not_and_parentheses_bind_as_written() {
    assert!(matches(
        OPEN_ISSUE,
        "not (fm.status == \"closed\" or fm.bug == \"B-99\")"
    ));
    assert!(!matches(
        OPEN_ISSUE,
        "not fm.status == \"open\" or fm.bug == \"B-99\""
    ));
    assert!(matches(
        OPEN_ISSUE,
        "fm.bug == \"B-99\" or fm.status == \"open\" and fm.type == \"issue\""
    ));
}

#[test]
fn a_typo_reports_its_position() {
    let text = "fm.type = \"x\"";
    let error = Query::parse(text).unwrap_err();
    assert_eq!(error.span.start, text.find('=').unwrap());
}

#[test]
fn toml_and_yaml_files_answer_the_same_query() {
    let toml = "+++\ntype = \"task\"\nstatus = \"open\"\npriority = 2\n+++\n";
    let yaml = "---\ntype: task\nstatus: open\npriority: 2\n---\n";
    for query in [
        "fm.type == \"task\" and fm.priority <= 2",
        "fm.status == \"open\"",
        "fm.priority > 2",
    ] {
        assert_eq!(matches(toml, query), matches(yaml, query), "{query}");
    }
}
