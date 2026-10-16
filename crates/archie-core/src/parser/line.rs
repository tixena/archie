//! Splitting the source into lines.

/// One line of the source. `end` excludes the line ending; `next` is where
/// the following line starts.
pub struct Line {
    pub end: usize,
    pub next: usize,
    pub start: usize,
}

pub fn line_at(source: &str, start: usize) -> Line {
    let (raw_end, next) = source[start..]
        .find('\n')
        .map_or((source.len(), source.len()), |at| {
            (start + at, start + at + 1)
        });
    let end = if source[start..raw_end].ends_with('\r') {
        raw_end - 1
    } else {
        raw_end
    };
    Line { end, next, start }
}
