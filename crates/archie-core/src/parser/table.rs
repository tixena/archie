//! Tables, read whole: which lines form a table, and where its cells are.
//! Lining cells up in columns is not context-free, so a table reaches the
//! grammar as one token.

use core::ops::Range;

use crate::parser::ast::{Alignment, TableFormat};
use crate::parser::line::line_at;

pub struct TableToken {
    pub alignments: Vec<Alignment>,
    pub format: TableFormat,
    pub head: Vec<RowToken>,
    pub rows: Vec<RowToken>,
}

pub struct RowToken {
    pub cells: Vec<Range<usize>>,
    pub span: Range<usize>,
}

/// One line of a table: its text from `start` to `end`, the start of the
/// line it is on, and where the next line starts.
#[derive(Clone)]
struct TableLine {
    end: usize,
    line_start: usize,
    next: usize,
    start: usize,
}

impl TableLine {
    /// The column the line's text starts at.
    const fn indent(&self) -> usize {
        self.start - self.line_start
    }
}

/// The table whose first line holds `first`, read on from `next`. Every
/// later line is indented at least `level` columns. Returns the table, its
/// span, and where the line after it starts.
pub fn read(
    source: &str,
    line_start: usize,
    first: Range<usize>,
    next: usize,
    level: usize,
) -> Option<(TableToken, Range<usize>, usize)> {
    let head = TableLine {
        end: first.end,
        line_start,
        next,
        start: first.start,
    };
    pipe(source, &head, level)
        .or_else(|| grid(source, &head, level))
        .or_else(|| simple(source, &head, level))
}

/// The line starting at `at`, when it holds text and is indented at least
/// `level` columns.
fn line_after(source: &str, at: usize, level: usize) -> Option<TableLine> {
    if at >= source.len() {
        return None;
    }
    let line = line_at(source, at);
    let text = &source[line.start..line.end];
    let content = text.trim_start_matches(' ');
    let indent = text.len() - content.len();
    let trimmed = content.trim_end();
    (!trimmed.is_empty() && indent >= level).then(|| TableLine {
        end: line.start + indent + trimmed.len(),
        line_start: line.start,
        next: line.next,
        start: line.start + indent,
    })
}

/// A header row, a delimiter row such as `|:--|--:|`, and the rows after
/// them up to the first line without a pipe.
fn pipe(source: &str, head: &TableLine, level: usize) -> Option<(TableToken, Range<usize>, usize)> {
    let head_cells = pipe_cells(source, head)?;
    let delimiter = line_after(source, head.next, level)?;
    let delimiter_cells = pipe_cells(source, &delimiter)?;
    if delimiter_cells.len() != head_cells.len() {
        return None;
    }
    let alignments = delimiter_cells
        .iter()
        .map(|cell| alignment(&source[cell.clone()], '-'))
        .collect::<Option<Vec<_>>>()?;
    let mut rows = Vec::new();
    let mut last = delimiter;
    while let Some(line) = line_after(source, last.next, level) {
        let Some(cells) = pipe_cells(source, &line) else {
            break;
        };
        if cells.len() != alignments.len() {
            return None;
        }
        rows.push(RowToken {
            cells,
            span: line.start..line.end,
        });
        last = line;
    }
    let table = TableToken {
        alignments,
        format: TableFormat::Pipe,
        head: vec![RowToken {
            cells: head_cells,
            span: head.start..head.end,
        }],
        rows,
    };
    Some((table, head.start..last.end, last.next))
}

/// The cells of a pipe row: the text between unescaped pipes, with an
/// outer pipe at either end optional. `None` when the line has no pipe.
fn pipe_cells(source: &str, line: &TableLine) -> Option<Vec<Range<usize>>> {
    let text = &source[line.start..line.end];
    let bytes = text.as_bytes();
    let mut pipes = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'|' => {
                pipes.push(at);
                at += 1;
            }
            _ => at += 1,
        }
    }
    let (&first, &last) = pipes.first().zip(pipes.last())?;
    let begin = usize::from(first == 0);
    let finish = if last + 1 == text.len() && last >= begin {
        last
    } else {
        text.len()
    };
    let walls: Vec<usize> = pipes
        .into_iter()
        .filter(|&pipe| pipe >= begin && pipe < finish)
        .collect();
    let mut cells = Vec::new();
    let mut from = begin;
    for wall in walls.into_iter().chain([finish]) {
        cells.push(trim(source, line.start + from..line.start + wall));
        from = wall + 1;
    }
    Some(cells)
}

/// A table drawn with `+---+` borders and `|` walls, every row one line.
/// A `+===+` border ends the header.
fn grid(source: &str, head: &TableLine, level: usize) -> Option<(TableToken, Range<usize>, usize)> {
    let top: Vec<char> = source[head.start..head.end].chars().collect();
    let (corners, mut alignments) = border(&top, '-')?;
    let mut head_rows = Vec::new();
    let mut rows = Vec::new();
    let mut last = head.clone();
    while let Some(line) = line_after(source, last.next, level) {
        if line.indent() != head.indent() || !source[line.start..line.end].starts_with('|') {
            break;
        }
        let cells = grid_cells(source, &line, &corners)?;
        rows.push(RowToken {
            cells,
            span: line.start..line.end,
        });
        let closing = line_after(source, line.next, level)
            .filter(|closing| closing.indent() == head.indent())?;
        let closing_chars: Vec<char> = source[closing.start..closing.end].chars().collect();
        if let Some((found, header)) = border(&closing_chars, '=') {
            if found != corners || !head_rows.is_empty() {
                return None;
            }
            head_rows.append(&mut rows);
            alignments = header;
        } else {
            let (found, _) = border(&closing_chars, '-')?;
            if found != corners {
                return None;
            }
        }
        last = closing;
    }
    if rows.is_empty() && head_rows.is_empty() {
        return None;
    }
    let table = TableToken {
        alignments,
        format: TableFormat::Grid,
        head: head_rows,
        rows,
    };
    Some((table, head.start..last.end, last.next))
}

/// The columns of a `+` corner in a border line made of `fill`, and the
/// alignment each segment's colons set.
fn border(chars: &[char], fill: char) -> Option<(Vec<usize>, Vec<Alignment>)> {
    if chars.len() < 3 || chars.first() != Some(&'+') || chars.last() != Some(&'+') {
        return None;
    }
    let corners: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter_map(|(column, &character)| (character == '+').then_some(column))
        .collect();
    let alignments = corners
        .iter()
        .zip(corners.iter().skip(1))
        .map(|(&from, &to)| {
            let segment: String = chars[from + 1..to].iter().collect();
            alignment(&segment, fill)
        })
        .collect::<Option<Vec<_>>>()?;
    Some((corners, alignments))
}

/// The cells of a grid row, whose walls stand at the border's corners.
fn grid_cells(source: &str, line: &TableLine, corners: &[usize]) -> Option<Vec<Range<usize>>> {
    let text = &source[line.start..line.end];
    let offsets: Vec<(usize, char)> = text.char_indices().collect();
    if offsets.len() != corners.last().copied()? + 1 {
        return None;
    }
    if corners
        .iter()
        .any(|&column| offsets.get(column).map(|&(_, character)| character) != Some('|'))
    {
        return None;
    }
    let cells = corners
        .iter()
        .zip(corners.iter().skip(1))
        .map(|(&left, &right)| {
            let from = offsets[left].0 + 1;
            let to = offsets[right].0;
            trim(source, line.start + from..line.start + to)
        })
        .collect();
    Some(cells)
}

/// A header line, a line of dash groups marking the columns, and the rows
/// after them up to a blank line. No text may cross from one column into
/// the next.
fn simple(
    source: &str,
    head: &TableLine,
    level: usize,
) -> Option<(TableToken, Range<usize>, usize)> {
    let dashes = line_after(source, head.next, level)?;
    let groups = dash_groups(&source[dashes.line_start..dashes.end])?;
    let mut lines = vec![head.clone()];
    let mut last = dashes;
    while let Some(line) = line_after(source, last.next, level) {
        lines.push(line.clone());
        last = line;
    }
    if lines.len() < 2 {
        return None;
    }
    let mut rows = lines
        .iter()
        .map(|line| simple_row(source, line, &groups))
        .collect::<Option<Vec<_>>>()?;
    let head_row = rows.remove(0);
    let alignments = head_row
        .cells
        .iter()
        .zip(&groups)
        .map(|(cell, group)| {
            let left = column_of(source, head.line_start, cell.start);
            let right = column_of(source, head.line_start, cell.end);
            match (left == group.start, right == group.end) {
                _ if cell.is_empty() => Alignment::Default,
                (true, true) => Alignment::Default,
                (true, false) => Alignment::Left,
                (false, true) => Alignment::Right,
                (false, false) => Alignment::Center,
            }
        })
        .collect();
    let table = TableToken {
        alignments,
        format: TableFormat::Simple,
        head: vec![head_row],
        rows,
    };
    Some((table, head.start..last.end, last.next))
}

/// The columns, counted from the start of the line, of each run of dashes
/// in a line made only of dashes and spaces. At least two runs.
fn dash_groups(text: &str) -> Option<Vec<Range<usize>>> {
    if !text
        .chars()
        .all(|character| character == '-' || character == ' ')
    {
        return None;
    }
    let mut groups: Vec<Range<usize>> = Vec::new();
    for (column, character) in text.chars().enumerate() {
        if character != '-' {
            continue;
        }
        match groups.last_mut() {
            Some(group) if group.end == column => group.end = column + 1,
            Some(_) | None => groups.push(column..column + 1),
        }
    }
    (groups.len() >= 2).then_some(groups)
}

/// The cells of a simple row. Each column runs from its dash group to the
/// next group; text before the first group, or a word across a column
/// edge, makes the row malformed.
fn simple_row(source: &str, line: &TableLine, groups: &[Range<usize>]) -> Option<RowToken> {
    let text = &source[line.line_start..line.end];
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let first = groups.first()?.start;
    if chars
        .iter()
        .take(first)
        .any(|&(_, character)| character != ' ')
    {
        return None;
    }
    let edges: Vec<usize> = groups.iter().skip(1).map(|group| group.start).collect();
    for &edge in &edges {
        let before = chars.get(edge - 1).map(|&(_, character)| character);
        let after = chars.get(edge).map(|&(_, character)| character);
        if before.is_some_and(|character| character != ' ')
            && after.is_some_and(|character| character != ' ')
        {
            return None;
        }
    }
    let byte_at = |column: usize| chars.get(column).map_or(text.len(), |&(offset, _)| offset);
    let starts = groups.iter().map(|group| group.start);
    let ends = edges.iter().copied().chain([chars.len()]);
    let cells = starts
        .zip(ends)
        .map(|(from, to)| {
            trim(
                source,
                line.line_start + byte_at(from)..line.line_start + byte_at(to),
            )
        })
        .collect();
    Some(RowToken {
        cells,
        span: line.start..line.end,
    })
}

/// The column of byte `at` on the line starting at `line_start`.
fn column_of(source: &str, line_start: usize, at: usize) -> usize {
    source[line_start..at].chars().count()
}

/// The alignment a cell such as `:--:` sets: a colon at the left, the
/// right or both, around one or more `fill` characters.
fn alignment(text: &str, fill: char) -> Option<Alignment> {
    let left = text.starts_with(':');
    let right = text.len() > 1 && text.ends_with(':');
    let opened = text.strip_prefix(':').unwrap_or(text);
    let inner = opened.strip_suffix(':').unwrap_or(opened);
    if inner.is_empty() || !inner.chars().all(|character| character == fill) {
        return None;
    }
    Some(match (left, right) {
        (true, true) => Alignment::Center,
        (true, false) => Alignment::Left,
        (false, true) => Alignment::Right,
        (false, false) => Alignment::Default,
    })
}

/// `range` without the spaces and tabs at either end.
fn trim(source: &str, range: Range<usize>) -> Range<usize> {
    let text = &source[range.clone()];
    let start = range.start + (text.len() - text.trim_start_matches([' ', '\t']).len());
    let end = range.end - (text.len() - text.trim_end_matches([' ', '\t']).len());
    if start > end {
        start..start
    } else {
        start..end
    }
}
