//! Inline nodes written on a line: markup as archie writes it, and text with
//! a backslash before each character that would otherwise read as markup.

use crate::parser::ast::{Code, Continues, Emphasis, Image, Inline, Link, LinkForm, Strong};
use crate::parser::definition;
use crate::parser::entity;
use crate::parser::extension::{Construct, Extension};
use crate::parser::inline::{is_punctuation, is_whitespace};
use crate::parser::table;
use crate::writer::Writer;

/// What is written next to a text: a character, the start or the end of its
/// line, or nothing known, for a node written on its own.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Near {
    Char(char),
    Edge,
    Unknown,
}

/// The backticks around a code span, the same for every piece of a span
/// written over several lines, and whether a space pads the content inside
/// them.
#[derive(Clone, Copy)]
struct Fence {
    pad: bool,
    ticks: usize,
}

/// The code span pieces of a paragraph or a heading, in the order they are
/// written: each piece's place among the whole spans, and each whole span's
/// content.
#[derive(Default)]
pub struct Pieces {
    open: bool,
    owners: Vec<usize>,
    spans: Vec<String>,
}

/// A part of a line: markup, written as it is, or text, escaped by what is
/// around it. `link` marks text between a link's brackets.
enum Segment {
    Markup(String),
    Text { link: bool, value: String },
}

/// The segments of the line being written, and the fence of each code span
/// piece still to write, the next one last.
pub struct Segments {
    fences: Vec<Fence>,
    list: Vec<Segment>,
    /// Whether every `|` of the text is escaped.
    pipes: bool,
}

/// What a run of `*` or `_` has on one side.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Mark,
    Other,
    Space,
}

impl Fence {
    /// A run of backticks longer than any inside `value`, padded when the
    /// value starts or ends with a backtick, or starts and ends with a
    /// space and is not all spaces, since the reader strips one space from
    /// each end.
    fn of(value: &str) -> Self {
        let blank = value.bytes().all(|byte| matches!(byte, b' ' | b'\n'));
        let pad = value.starts_with('`')
            || value.ends_with('`')
            || (!blank && value.starts_with(' ') && value.ends_with(' '));
        Self {
            pad,
            ticks: super::longest_run(value) + 1,
        }
    }
}

impl Pieces {
    /// Adds the code span pieces among `nodes` and their children.
    pub fn add<T>(&mut self, nodes: &[Inline<T>])
    where
        T: Extension,
    {
        for node in nodes {
            match node {
                Inline::Code(code) if code.kind.0.is_none() => self.code(code),
                Inline::Emphasis(emphasis) if emphasis.kind.0.is_none() => {
                    self.add(&emphasis.children);
                }
                Inline::Link(link) if link.kind.0.is_none() && link.form == LinkForm::Markdown => {
                    self.add(&link.children);
                }
                Inline::Strong(strong) if strong.kind.0.is_none() => self.add(&strong.children),
                Inline::Code(_)
                | Inline::Emphasis(_)
                | Inline::Html(_)
                | Inline::Image(_)
                | Inline::LineBreak(_)
                | Inline::Link(_)
                | Inline::Strong(_)
                | Inline::Text(_) => {}
            }
        }
    }

    /// A piece that goes on from the line before joins the span it is part
    /// of.
    fn code<T>(&mut self, code: &Code<T>)
    where
        T: Extension,
    {
        let continued = matches!(code.continues, Continues::Both | Continues::Previous);
        match self.spans.last_mut() {
            Some(span) if continued && self.open => {
                span.push('\n');
                span.push_str(&code.value);
            }
            Some(_) | None => self.spans.push(code.value.clone()),
        }
        self.owners.push(self.spans.len().saturating_sub(1));
        self.open = matches!(code.continues, Continues::Both | Continues::Next);
    }

    /// The fence of each piece, the first one last.
    fn fences(&self) -> Vec<Fence> {
        self.owners
            .iter()
            .rev()
            .map(|&owner| Fence::of(&self.spans[owner]))
            .collect()
    }

    /// The code span pieces among `nodes`.
    pub fn of<T>(nodes: &[Inline<T>]) -> Self
    where
        T: Extension,
    {
        let mut pieces = Self::default();
        pieces.add(nodes);
        pieces
    }
}

impl Segment {
    /// The character the segment starts with, or ends with when `last` is
    /// set.
    fn edge(&self, last: bool) -> Near {
        let text = match self {
            Self::Markup(markup) => markup,
            Self::Text { link: _, value } => value,
        };
        let found = if last {
            text.chars().next_back()
        } else {
            text.chars().next()
        };
        found.map_or(Near::Unknown, Near::Char)
    }
}

impl Segments {
    /// A code span piece: the opening backticks before the first piece, the
    /// closing ones after the last.
    pub fn code<T>(&mut self, code: &Code<T>)
    where
        T: Extension,
    {
        let fence = self.fences.pop().unwrap_or_else(|| Fence::of(&code.value));
        let ticks = "`".repeat(fence.ticks);
        let pad = if fence.pad { " " } else { "" };
        let mut written = String::new();
        if opens(code.continues) {
            written.push_str(&ticks);
            written.push_str(pad);
        }
        written.push_str(&code.value);
        if closes(code.continues) {
            written.push_str(pad);
            written.push_str(&ticks);
        }
        self.markup(&written);
    }

    /// `*emphasis*`.
    pub fn emphasis<T>(&mut self, writer: &Writer, emphasis: &Emphasis<T>, link: bool)
    where
        T: Extension,
    {
        self.wrapped(writer, &emphasis.children, emphasis.continues, "*", link);
    }

    /// Puts a backslash before every `|` of the text written from now on.
    pub const fn escape_pipes(&mut self) {
        self.pipes = true;
    }

    /// `![alt](target "title")`, or `![[target|alt]]` for a wiki embed.
    pub fn image<T>(&mut self, image: &Image<T>)
    where
        T: Extension,
    {
        let mut written = String::new();
        match image.form {
            LinkForm::Markdown => {
                if opens(image.continues) {
                    written.push_str("![");
                }
                written.push_str(&image.alt);
                if closes(image.continues) {
                    written.push_str(&target(&image.target, image.title.as_deref()));
                }
            }
            LinkForm::Wiki => {
                written.push_str("![[");
                written.push_str(&image.target);
                if !image.alt.is_empty() {
                    written.push('|');
                    written.push_str(&image.alt);
                }
                written.push_str("]]");
            }
        }
        self.markup(&written);
    }

    /// The line written, with a backslash before a first character that
    /// would start a block, and whether it starts with text. `first` is set
    /// for the first line of a paragraph.
    pub fn line(&mut self, first: bool) -> (String, bool) {
        let text_led = matches!(self.list.first(), Some(Segment::Text { link: _, value: _ }));
        let mut text = self.take(Near::Edge, Near::Edge);
        if text_led && let Some(at) = block_start(&text, first) {
            text.insert(at, '\\');
        }
        (text, text_led)
    }

    /// `[text](target "title")`, also for a reference link or an autolink,
    /// or `[[target|text]]` for a wiki link. The `[` goes before the first
    /// piece of a link written over several lines, the rest after the last.
    pub fn link<T>(&mut self, writer: &Writer, link: &Link<T>)
    where
        T: Extension,
    {
        match link.form {
            LinkForm::Markdown => {
                if opens(link.continues) {
                    self.markup("[");
                }
                self.nodes(writer, &link.children, true);
                if closes(link.continues) {
                    self.markup(&target(&link.target, link.title.as_deref()));
                }
            }
            LinkForm::Wiki => {
                let mut written = format!("[[{}", link.target);
                let alias: String = link.children.iter().filter_map(text_of).collect();
                if !alias.is_empty() {
                    written.push('|');
                    written.push_str(&alias);
                }
                written.push_str("]]");
                self.markup(&written);
            }
        }
    }

    /// Adds `text`, written as it is.
    pub fn markup(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        match self.list.last_mut() {
            Some(Segment::Markup(last)) => last.push_str(text),
            Some(Segment::Text { link: _, value: _ }) | None => {
                self.list.push(Segment::Markup(text.to_owned()));
            }
        }
    }

    pub fn new(pieces: &Pieces) -> Self {
        Self {
            fences: pieces.fences(),
            list: Vec::new(),
            pipes: false,
        }
    }

    /// Adds `node`: what the extension that claimed it writes, or else its
    /// plain form. `link` is set between a link's brackets.
    pub fn node<T>(&mut self, writer: &Writer, node: &Inline<T>, link: bool)
    where
        T: Extension,
    {
        let (kind, plain) = match node {
            Inline::Code(code) => (&code.kind, Construct::Code(code)),
            Inline::Emphasis(emphasis) => (&emphasis.kind, Construct::Emphasis(emphasis)),
            Inline::Html(html) => (&html.kind, Construct::Html(html)),
            Inline::Image(image) => (&image.kind, Construct::Image(image)),
            Inline::LineBreak(line_break) => (&line_break.kind, Construct::LineBreak(line_break)),
            Inline::Link(anchor) => (&anchor.kind, Construct::Link(anchor)),
            Inline::Strong(strong) => (&strong.kind, Construct::Strong(strong)),
            Inline::Text(text) => (&text.kind, Construct::Text(text)),
        };
        match kind.0.as_deref() {
            Some(claimed) => self.markup(&writer.claimed(claimed, plain)),
            None => self.plain(writer, node, link),
        }
    }

    pub fn nodes<T>(&mut self, writer: &Writer, nodes: &[Inline<T>], link: bool)
    where
        T: Extension,
    {
        for node in nodes {
            self.node(writer, node, link);
        }
    }

    fn plain<T>(&mut self, writer: &Writer, node: &Inline<T>, link: bool)
    where
        T: Extension,
    {
        match node {
            Inline::Code(code) => self.code(code),
            Inline::Emphasis(emphasis) => self.emphasis(writer, emphasis, link),
            Inline::Html(html) => self.markup(&html.value),
            Inline::Image(image) => self.image(image),
            Inline::LineBreak(_) => self.markup("\\"),
            Inline::Link(anchor) => self.link(writer, anchor),
            Inline::Strong(strong) => self.strong(writer, strong, link),
            Inline::Text(text) => self.text(&text.value, link),
        }
    }

    /// `**strong**`.
    pub fn strong<T>(&mut self, writer: &Writer, strong: &Strong<T>, link: bool)
    where
        T: Extension,
    {
        self.wrapped(writer, &strong.children, strong.continues, "**", link);
    }

    /// The segments written between `start` and `end`, which are taken out.
    pub fn take(&mut self, start: Near, end: Near) -> String {
        let mut out = String::new();
        for (index, segment) in self.list.iter().enumerate() {
            match segment {
                Segment::Markup(markup) => out.push_str(markup),
                Segment::Text { link, value } => {
                    let before = index
                        .checked_sub(1)
                        .and_then(|previous| self.list.get(previous))
                        .map_or(start, |previous| previous.edge(true));
                    let after = self
                        .list
                        .get(index + 1)
                        .map_or(end, |next| next.edge(false));
                    escape(&mut out, value, before, after, *link, self.pipes);
                }
            }
        }
        self.list.clear();
        out
    }

    /// Adds text, to be escaped by what is around it.
    pub fn text(&mut self, value: &str, link: bool) {
        if value.is_empty() {
            return;
        }
        match self.list.last_mut() {
            Some(Segment::Text {
                link: last_link,
                value: last,
            }) if *last_link == link => last.push_str(value),
            Some(Segment::Markup(_) | Segment::Text { link: _, value: _ }) | None => {
                self.list.push(Segment::Text {
                    link,
                    value: value.to_owned(),
                });
            }
        }
    }

    /// `children` between two `mark`s: the opening one before the first
    /// piece of a node written over several lines, the closing one after
    /// the last.
    fn wrapped<T>(
        &mut self,
        writer: &Writer,
        children: &[Inline<T>],
        continues: Continues,
        mark: &str,
        link: bool,
    ) where
        T: Extension,
    {
        if opens(continues) {
            self.markup(mark);
        }
        self.nodes(writer, children, link);
        if closes(continues) {
            self.markup(mark);
        }
    }
}

/// Puts a backslash before the `[` that starts the first of `lines`, a
/// paragraph that starts with text, when the paragraph would read as a link
/// reference definition.
pub fn guard_definition(lines: &mut [String]) {
    let defines = lines.first().is_some_and(|first| first.starts_with('['))
        && definition::read(&lines.join("\n")).definition.is_some();
    if defines && let Some(first) = lines.first_mut() {
        first.insert(0, '\\');
    }
}

/// Whether `lines`, a paragraph as written, would read as a table that
/// starts on one of them, each line offered to the table reader as the
/// lexer offers it.
pub fn reads_as_table(lines: &[String]) -> bool {
    let source = lines.join("\n");
    let mut line_start = 0;
    for line in lines {
        let content = line.trim_start_matches(' ');
        let start = line_start + (line.len() - content.len());
        let end = start + content.trim_end_matches([' ', '\t']).len();
        let next = (line_start + line.len() + 1).min(source.len());
        if table::read(&source, line_start, start..end, next, 0, 0).is_some() {
            return true;
        }
        line_start = next;
    }
    false
}

/// Where a backslash goes so that `text`, a line that starts with text, does
/// not start a block: before the first character of a heading, a block
/// quote, a bullet, a thematic break, a tilde fence or, after the first line
/// of a paragraph, a heading underline; before the `.` or `)` of an ordered
/// list's number.
fn block_start(text: &str, first: bool) -> Option<usize> {
    let bytes = text.as_bytes();
    let marks = bytes.iter().take_while(|&&byte| byte == b'#').count();
    let heading = (1..=6).contains(&marks) && matches!(bytes.get(marks), None | Some(b' '));
    let bullet = matches!(bytes.first(), Some(b'-' | b'+' | b'*'))
        && matches!(bytes.get(1), None | Some(b' '));
    let underline = !first
        && bytes.first().is_some_and(|&mark| {
            matches!(mark, b'=' | b'-') && bytes.iter().all(|&byte| byte == mark)
        });
    let fence = text.starts_with("~~~");
    if heading || bullet || underline || fence || is_thematic_break(text) || text.starts_with('>') {
        return Some(0);
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    ((1..=9).contains(&digits)
        && matches!(bytes.get(digits), Some(b'.' | b')'))
        && matches!(bytes.get(digits + 1), None | Some(b' ')))
    .then_some(digits)
}

/// Whether a piece that `continues` closes on its line.
const fn closes(continues: Continues) -> bool {
    matches!(continues, Continues::Neither | Continues::Previous)
}

/// Writes `value` with a backslash before each character that is not to be
/// read as markup: a backslash before punctuation or a line end, a
/// backtick, a `*` or `_` run that could open or close emphasis, a `&` that
/// starts a reference, a `<` that could start a tag or an autolink, and the
/// `!`, `[` and `]` that would start or end a link. Between a link's
/// brackets, `link`, every bracket is escaped, and with `pipes`, every `|`.
fn escape(out: &mut String, value: &str, before: Near, after: Near, link: bool, pipes: bool) {
    let chars: Vec<(usize, char)> = value.char_indices().collect();
    let near = |index: usize| {
        chars
            .get(index)
            .map_or(after, |&(_, character)| Near::Char(character))
    };
    let mut index = 0;
    while let Some(&(at, character)) = chars.get(index) {
        let previous = index
            .checked_sub(1)
            .map_or(before, |back| Near::Char(chars[back].1));
        let (end, escaped) = if matches!(character, '*' | '_') {
            let end = index
                + chars[index..]
                    .iter()
                    .take_while(|&&(_, next)| next == character)
                    .count();
            (end, !inert(character, previous, near(end)))
        } else {
            let next = near(index + 1);
            let escaped = match character {
                '\\' => !matches!(next, Near::Char(found) if !found.is_ascii_punctuation()),
                '`' => true,
                '&' => entity::reference(&value[at..]).is_some(),
                '<' => {
                    !matches!(next, Near::Edge)
                        && !matches!(next, Near::Char(found) if is_whitespace(found))
                }
                '[' => link || matches!(next, Near::Char('[') | Near::Unknown),
                ']' => link || matches!(next, Near::Char('(') | Near::Unknown),
                '!' => matches!(next, Near::Char('[') | Near::Unknown),
                '|' => pipes,
                _ => false,
            };
            (index + 1, escaped)
        };
        for &(_, written) in &chars[index..end] {
            if escaped {
                out.push('\\');
            }
            out.push(written);
        }
        index = end;
    }
}

/// Whether a run of `character`, `*` or `_`, between `before` and `after`
/// can neither open nor close emphasis, by the reader's flanking rules, so
/// it reads as text. A side that is not known, or is the same character,
/// counts as one that could.
fn inert(character: char, before: Near, after: Near) -> bool {
    let (Some(left_side), Some(right_side)) = (side(before, character), side(after, character))
    else {
        return false;
    };
    let space_before = left_side == Side::Space;
    let space_after = right_side == Side::Space;
    let mark_before = left_side == Side::Mark;
    let mark_after = right_side == Side::Mark;
    let left = !space_after && (!mark_after || space_before || mark_before);
    let right = !space_before && (!mark_before || space_after || mark_after);
    let (can_open, can_close) = if character == '*' {
        (left, right)
    } else {
        (
            left && (!right || mark_before),
            right && (!left || mark_after),
        )
    };
    !can_open && !can_close
}

/// Three or more of the same `-`, `*` or `_`, with nothing but spaces
/// between them.
fn is_thematic_break(text: &str) -> bool {
    text.chars()
        .next()
        .filter(|mark| matches!(mark, '-' | '*' | '_'))
        .is_some_and(|mark| {
            text.chars()
                .all(|character| character == mark || character == ' ')
                && text.chars().filter(|&character| character == mark).count() >= 3
        })
}

/// Whether a piece that `continues` opens on its line.
const fn opens(continues: Continues) -> bool {
    matches!(continues, Continues::Neither | Continues::Next)
}

/// What `near` is to a run of `character`, or `None` when it is not known
/// or is the same character, which would join the run.
fn side(near: Near, character: char) -> Option<Side> {
    match near {
        Near::Char(found) if found == character => None,
        Near::Char(found) if is_whitespace(found) => Some(Side::Space),
        Near::Char(found) if is_punctuation(found) => Some(Side::Mark),
        Near::Char(_) => Some(Side::Other),
        Near::Edge => Some(Side::Space),
        Near::Unknown => None,
    }
}

/// `](destination "title")`, the end of a link or an image. A destination
/// with a space or a control character, one that starts with `<`, or one
/// whose parentheses do not pair up goes between `<` and `>`.
fn target(destination: &str, title: Option<&str>) -> String {
    let mut written = "](".to_owned();
    if destination.is_empty() {
        if title.is_some() {
            written.push_str("<>");
        }
    } else if destination.starts_with('<')
        || destination.bytes().any(|byte| byte <= b' ' || byte == 0x7f)
        || !paired(destination)
    {
        written.push('<');
        escape_marks(&mut written, destination, &['<', '>']);
        written.push('>');
    } else {
        escape_marks(&mut written, destination, &[]);
    }
    if let Some(text) = title {
        written.push_str(" \"");
        escape_marks(&mut written, text, &['"']);
        written.push('"');
    }
    written.push(')');
    written
}

/// Writes `text`, a destination or a title, with a backslash before each of
/// `marks`, each `&` that starts a reference, and each backslash before
/// punctuation or the end.
fn escape_marks(out: &mut String, text: &str, marks: &[char]) {
    let mut chars = text.char_indices().peekable();
    while let Some((at, character)) = chars.next() {
        let escaped = match character {
            '\\' => chars
                .peek()
                .is_none_or(|&(_, next)| next.is_ascii_punctuation()),
            '&' => entity::reference(&text[at..]).is_some(),
            _ => marks.contains(&character),
        };
        if escaped {
            out.push('\\');
        }
        out.push(character);
    }
}

/// Whether the parentheses of `destination` pair up, none closing before it
/// opens.
fn paired(destination: &str) -> bool {
    let mut depth = 0_usize;
    for character in destination.chars() {
        match character {
            '(' => depth += 1,
            ')' => match depth.checked_sub(1) {
                Some(inner) => depth = inner,
                None => return false,
            },
            _ => {}
        }
    }
    depth == 0
}

/// The text of `node`, when it is text.
fn text_of<T>(node: &Inline<T>) -> Option<&str>
where
    T: Extension,
{
    match node {
        Inline::Text(text) => Some(&text.value),
        Inline::Code(_)
        | Inline::Emphasis(_)
        | Inline::Html(_)
        | Inline::Image(_)
        | Inline::LineBreak(_)
        | Inline::Link(_)
        | Inline::Strong(_) => None,
    }
}
