use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::page::page;

fn cursor_of(key: &str) -> String {
    URL_SAFE_NO_PAD.encode(key)
}

fn keys(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn the_first_page_starts_at_the_beginning() {
    let all = keys(&["a", "b", "c", "d", "e"]);
    let (items, next) = page(&all, None, 2);
    assert_eq!(items, ["a", "b"]);
    assert_eq!(next, Some(cursor_of("b")));
}

#[test]
fn next_page_continues_after_the_cursor() {
    let all = keys(&["a", "b", "c", "d", "e"]);
    let (items, next) = page(&all, Some(&cursor_of("b")), 2);
    assert_eq!(items, ["c", "d"]);
    assert_eq!(next, Some(cursor_of("d")));
}

#[test]
fn the_last_page_has_no_cursor() {
    let all = keys(&["a", "b", "c", "d", "e"]);
    let (items, next) = page(&all, Some(&cursor_of("d")), 2);
    assert_eq!(items, ["e"]);
    assert_eq!(next, None);
    let (exact, exact_next) = page(&all, Some(&cursor_of("c")), 2);
    assert_eq!(exact, ["d", "e"]);
    assert_eq!(exact_next, None);
}

#[test]
fn a_key_deleted_between_pages_does_not_repeat_or_skip_others() {
    let all = keys(&["a", "b", "c", "d", "e"]);
    let (first, next) = page(&all, None, 2);
    assert_eq!(first, ["a", "b"]);
    let after_delete = keys(&["a", "c", "d", "e"]);
    let (second, _) = page(&after_delete, next.as_deref(), 2);
    assert_eq!(second, ["c", "d"]);
}

#[test]
fn a_garbage_cursor_starts_from_the_beginning() {
    let all = keys(&["a", "b", "c"]);
    let (not_base64, base64_next) = page(&all, Some("!!!not base64!!!"), 2);
    assert_eq!(not_base64, ["a", "b"]);
    assert_eq!(base64_next, Some(cursor_of("b")));
    let (not_utf8, utf8_next) = page(&all, Some(&URL_SAFE_NO_PAD.encode([0xff, 0xfe])), 2);
    assert_eq!(not_utf8, ["a", "b"]);
    assert_eq!(utf8_next, Some(cursor_of("b")));
}

#[test]
fn a_zero_limit_still_returns_one_key_and_a_cursor() {
    let all = keys(&["a", "b", "c"]);
    let (items, next) = page(&all, None, 0);
    assert_eq!(items, ["a"]);
    assert_eq!(next, Some(cursor_of("a")));
}

#[test]
fn an_empty_key_list_gives_an_empty_last_page() {
    let (items, next) = page(&[], None, 3);
    assert!(items.is_empty());
    assert_eq!(next, None);
}

#[test]
fn non_ascii_keys_round_trip_through_the_cursor() {
    let all = keys(&["a", "\u{e9}", "\u{65e5}\u{672c}"]);
    let (first, first_next) = page(&all, None, 1);
    assert_eq!(first, ["a"]);
    let (second, second_next) = page(&all, first_next.as_deref(), 1);
    assert_eq!(second, ["\u{e9}"]);
    assert_eq!(second_next, Some(cursor_of("\u{e9}")));
    let (third, third_next) = page(&all, second_next.as_deref(), 1);
    assert_eq!(third, ["\u{65e5}\u{672c}"]);
    assert_eq!(third_next, None);
}

#[test]
fn a_cursor_past_the_last_key_gives_an_empty_last_page() {
    let all = keys(&["a", "b"]);
    let (items, next) = page(&all, Some(&cursor_of("z")), 2);
    assert!(items.is_empty());
    assert_eq!(next, None);
}
