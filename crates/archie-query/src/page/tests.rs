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
    let without_cursor_key = keys(&["a", "c", "d", "e"]);
    let (third, _) = page(&without_cursor_key, Some(&cursor_of("b")), 2);
    assert_eq!(third, ["c", "d"]);
}

#[test]
fn a_garbage_cursor_starts_from_the_beginning() {
    let all = keys(&["a", "b", "c"]);
    let (not_base64, _) = page(&all, Some("!!!not base64!!!"), 2);
    assert_eq!(not_base64, ["a", "b"]);
    let (not_utf8, _) = page(&all, Some(&URL_SAFE_NO_PAD.encode([0xff, 0xfe])), 2);
    assert_eq!(not_utf8, ["a", "b"]);
}
