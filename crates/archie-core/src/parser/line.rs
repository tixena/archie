//! Splitting the source into lines, and counting the columns of the white
//! space that starts a text.

/// One line of the source. `end` excludes the line ending; `next` is where
/// the following line starts. Its tab stops count from `origin` columns
/// before its column 0, and `start` is at its column `column`.
#[derive(Clone, Copy)]
pub struct Line {
    pub column: usize,
    pub end: usize,
    pub next: usize,
    pub origin: usize,
    pub start: usize,
}

/// What each line is read past: at most `columns` columns of the white
/// space that starts it. Its tab stops count from its column 0 when
/// `restart` is set, or else from the start of the source line.
#[derive(Clone, Copy, Default)]
pub struct Base {
    pub columns: usize,
    pub restart: bool,
}

/// The white space that starts a text: the bytes it takes and the columns
/// it spans.
#[derive(Clone, Copy)]
pub struct Space {
    pub bytes: usize,
    pub columns: usize,
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
    Line {
        column: 0,
        end,
        next,
        origin: 0,
        start,
    }
}

/// The line at `start`, read past `base`. A tab that `base.columns` falls
/// inside is passed, and its columns past them are the line's first
/// columns, before `start`.
pub fn line_from(source: &str, start: usize, base: Base) -> Line {
    let line = line_at(source, start);
    let left_out = space_to(&source[line.start..line.end], 0, base.columns);
    let origin = left_out.columns.min(base.columns);
    Line {
        column: left_out.columns - origin,
        origin: if base.restart { 0 } else { origin },
        start: line.start + left_out.bytes,
        ..line
    }
}

/// The spaces and tabs that start `text`, whose first byte is at `column` of
/// its source line, a tab moving to the next multiple of 4.
pub fn space(text: &str, column: usize) -> Space {
    space_to(text, column, usize::MAX)
}

/// The spaces and tabs that start `text`, whose first byte is at `column` of
/// its source line, up to column `limit`, counted as `space` counts them. A
/// tab that `limit` falls inside is taken whole.
pub fn space_to(text: &str, column: usize, limit: usize) -> Space {
    let mut reached = column;
    let mut bytes = 0;
    for byte in text.bytes() {
        if reached >= limit {
            break;
        }
        reached = match byte {
            b' ' => reached + 1,
            b'\t' => next_stop(reached),
            _ => break,
        };
        bytes += 1;
    }
    Space {
        bytes,
        columns: reached - column,
    }
}

/// The column that `text`, written from the start of its source line, ends
/// at: a tab moves to the next multiple of 4, and any other character one
/// column on.
pub fn width(text: &str) -> usize {
    text.chars().fold(0, |column, character| {
        if character == '\t' {
            next_stop(column)
        } else {
            column + 1
        }
    })
}

/// The column a tab at `column` moves to.
const fn next_stop(column: usize) -> usize {
    column + 4 - column % 4
}
