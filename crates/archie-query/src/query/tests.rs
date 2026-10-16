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
    for (query, expected) in [
        ("fm.type == \"task\" and fm.priority <= 2", true),
        ("fm.status == \"open\"", true),
        ("fm.priority > 2", false),
    ] {
        assert_eq!(matches(toml, query), expected, "toml: {query}");
        assert_eq!(matches(yaml, query), expected, "yaml: {query}");
    }
}

#[test]
fn and_binds_tighter_than_or() {
    assert!(matches(
        OPEN_ISSUE,
        "fm.bug == \"B-12\" or fm.status == \"closed\" and fm.type == \"x\""
    ));
    assert!(matches(
        OPEN_ISSUE,
        "fm.bug == \"B-99\" and fm.status == \"open\" or fm.type == \"issue\""
    ));
    assert!(!matches(
        OPEN_ISSUE,
        "fm.type == \"x\" or fm.status == \"closed\" and fm.bug == \"B-12\""
    ));
}

#[test]
fn keys_may_start_with_a_digit_or_be_the_word_has() {
    let source = "+++\n2fa = true\nhas = 3\nx-y = 1\n+++\n";
    assert!(matches(source, "fm.2fa == true"));
    assert!(matches(source, "fm.has == 3"));
    assert!(matches(source, "fm.x-y == 1"));
}

#[test]
fn an_int_and_a_float_compare_exactly() {
    let source =
        "+++\nbig = 5000000000\nmax = 9223372036854775807\nodd = 9007199254740993\nf = 5.5\n+++\n";
    assert!(matches(source, "fm.big > 1.5"));
    assert!(!matches(source, "fm.big < 1.5"));
    assert!(matches(source, "fm.big == 5000000000.0"));
    assert!(matches(source, "fm.max < 9223372036854775808.0"));
    assert!(matches(source, "fm.max > -9223372036854775808.0"));
    assert!(matches(source, "fm.max < 1e300"));
    assert!(matches(source, "fm.odd > 9007199254740992.0"));
    assert!(!matches(source, "fm.odd == 9007199254740992.0"));
    assert!(matches(source, "fm.f > 5"));
    assert!(matches(source, "fm.f < 6"));
    assert!(matches(source, "fm.f < 5000000000"));
    assert!(!matches(source, "fm.f == 5"));
    assert!(matches(source, "fm.f > -5000000000"));
}

#[test]
fn an_offset_datetime_compares_as_the_same_instant() {
    let source = "+++\nat = 2031-05-01T07:32:00-04:00\n+++\n";
    assert!(matches(source, "fm.at == 2031-05-01T11:32:00Z"));
    assert!(matches(source, "fm.at < 2031-05-01T11:32:01Z"));
}

#[test]
fn a_local_datetime_compares_as_utc() {
    let source = "+++\nat = 2031-05-01T11:32:00\n+++\n";
    assert!(matches(source, "fm.at == 2031-05-01T11:32:00Z"));
    assert!(matches(source, "fm.at == 2031-05-01T07:32:00-04:00"));
}

#[test]
fn an_unquoted_yaml_date_compares_as_a_date() {
    let yaml = "---\ndue: 2031-05-01\n---\n";
    assert!(matches(yaml, "fm.due == 2031-05-01"));
    assert!(matches(yaml, "fm.due < 2031-06-01"));
}

#[test]
fn a_query_nested_too_deep_is_refused() {
    let parens = format!("{}fm.a == 1{}", "(".repeat(65), ")".repeat(65));
    let error = Query::parse(&parens).unwrap_err();
    assert!(error.message.contains("nested"), "{error}");
    assert_eq!(error.span.start, 64);
    let nots = format!("{}fm.a == 1", "not ".repeat(65));
    assert!(Query::parse(&nots).unwrap_err().message.contains("nested"));
    let fine = format!("{}fm.a == 1{}", "(".repeat(64), ")".repeat(64));
    Query::parse(&fine).unwrap();
}

#[test]
fn a_query_with_too_many_terms_is_refused() {
    let many = vec!["fm.a == 1"; 257].join(" and ");
    let error = Query::parse(&many).unwrap_err();
    assert!(error.message.contains("too many"), "{error}");
    assert_eq!(error.span.start, many.rfind("fm.a").unwrap());
    let fine = vec!["fm.a == 1"; 256].join(" or ");
    Query::parse(&fine).unwrap();
}

#[test]
fn lexer_errors_span_what_they_name() {
    let unterminated = Query::parse("fm.a == \"open").unwrap_err();
    assert_eq!(unterminated.span, 8..13);
    let text = "fm.a == \u{e9}";
    let wide = Query::parse(text).unwrap_err();
    assert_eq!(wide.span, 8..10);
    let trailing = "fm.a == \"x\\";
    let backslash = Query::parse(trailing).unwrap_err();
    assert!(backslash.span.end <= trailing.len(), "{backslash:?}");
}

#[test]
fn a_string_with_a_toml_comment_is_not_a_date() {
    let yaml = "---\ndue: \"2031-05-01#anything\"\n---\n";
    assert!(!matches(yaml, "fm.due == 2031-05-01"));
    assert!(!matches(yaml, "fm.due < 2031-06-01"));
}

#[test]
fn toml_number_forms_parse_in_a_query() {
    let source = "+++\nn = 16\nf = 1000.0\n+++\n";
    assert!(matches(source, "fm.n == 0x10"));
    assert!(matches(source, "fm.n == 0o20"));
    assert!(matches(source, "fm.f == 1e3"));
    assert!(matches(source, "fm.n > -inf"));
}

#[test]
fn nan_never_matches() {
    let source = "+++\nn = 16\n+++\n";
    Query::parse("fm.n == -nan").unwrap();
    assert!(!matches(source, "fm.n == -nan"));
    assert!(!matches(source, "fm.n < -nan"));
    assert!(!matches(source, "fm.n > -nan"));
}

#[test]
fn a_trailing_dot_is_not_a_number() {
    Query::parse("fm.n == 5.").unwrap_err();
    Query::parse("fm.n == .5").unwrap_err();
}

#[test]
fn a_leap_day_exists_only_in_a_leap_year() {
    let source = "+++\nd = 2032-02-29\n+++\n";
    assert!(matches(source, "fm.d == 2032-02-29"));
    Query::parse("fm.d == 2031-02-29").unwrap_err();
}

#[test]
fn an_offset_with_sixty_minutes_is_refused() {
    Query::parse("fm.at == 2031-05-01T10:00:00+05:60").unwrap_err();
    Query::parse("fm.at == 2031-05-01T10:00:00+05:59").unwrap();
}
