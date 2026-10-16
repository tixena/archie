//! Cursor paging over sorted keys.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Keys sorted ascending by the caller. Cursor = base64url (no padding) of the last returned key.
///
/// The page holds up to `limit` keys (a `limit` of 0 is treated as 1, so a
/// missing cursor always means the end) that sort after the cursor's key, and the
/// returned cursor is `Some` only when more keys follow. The cursor key need
/// not exist any more, so a key deleted between pages repeats and skips
/// nothing. A cursor that does not decode is treated as no cursor.
#[must_use]
pub fn page<'keys>(
    keys: &'keys [String],
    after: Option<&str>,
    limit: usize,
) -> (Vec<&'keys str>, Option<String>) {
    let start = after
        .and_then(|cursor| URL_SAFE_NO_PAD.decode(cursor).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map_or(0, |last| keys.partition_point(|key| *key <= last));
    let rest = &keys[start..];
    let taken = &rest[..limit.max(1).min(rest.len())];
    let next = if taken.len() < rest.len() {
        taken.last().map(|last| URL_SAFE_NO_PAD.encode(last))
    } else {
        None
    };
    (taken.iter().map(String::as_str).collect(), next)
}

#[cfg(test)]
mod tests;
