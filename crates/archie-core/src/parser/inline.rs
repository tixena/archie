//! Inline nodes inside the text lines of a paragraph or a heading.

use core::mem;
use core::ops::{Range, RangeInclusive};
use std::collections::HashSet;

use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory as _};

use crate::parser::ast::{
    Code, Continues, Emphasis, Html, Image, Inline, Kind, Line, LineBreak, Link, LinkForm, Strong,
    Text,
};
use crate::parser::definition::{self, Definitions, Target, fold_label};
use crate::parser::extension::Extension;
use crate::parser::{entity, html};

/// What ends a line of a paragraph before the line after it.
#[derive(Clone, Copy)]
enum Ending {
    /// A backslash that ends the line is a hard line break.
    Backslash,
    /// A code span goes on into the next line: the spaces and tabs before
    /// the line ending, up to this place in the source, belong to it.
    Code(usize),
    /// No hard line break can end the line.
    Soft,
    /// Two or more spaces, up to this place in the source, are a hard line
    /// break.
    Spaces(usize),
}

/// The lines of a paragraph, `ranges` of `source`, joined by `\n` into
/// `text`, which the scanner reads: where each line sits in `text`, and how
/// it ends.
struct Layout<'src> {
    endings: Vec<Ending>,
    places: Vec<Range<usize>>,
    ranges: &'src [Range<usize>],
    source: &'src str,
    text: String,
}

/// What the scanner has read: a node, or a run of `*` or `_` not yet
/// paired, by its place in the scanner's runs.
enum Piece<E>
where
    E: Extension,
{
    Node(Inline<E>),
    Run(usize),
}

/// A run of `*` or `_`. `count` and `span` cover the characters not yet
/// paired; `length` is the run as written.
struct Run {
    can_close: bool,
    can_open: bool,
    character: char,
    count: usize,
    length: usize,
    span: Range<usize>,
}

/// Where the parts of a `[[target|alias]]` are.
struct Wiki {
    alias: Option<Range<usize>>,
    close: usize,
    target: Range<usize>,
}

/// Reads the text of a layout into nodes whose spans are places in that
/// text.
struct Scanner<'lay, 'defs, E>
where
    E: Extension,
{
    definitions: &'defs Definitions,
    /// The `[` of each text found to hold a link, which starts no link.
    holders: HashSet<usize>,
    layout: &'lay Layout<'lay>,
    /// Whether a link has been read: the text of a link holds none.
    linked: bool,
    pieces: Vec<Piece<E>>,
    runs: Vec<Run>,
    text: String,
    text_end: usize,
    text_start: Option<usize>,
}

impl Ending {
    /// How the line whose text ends at `end` in `source` ends: when another
    /// line of its paragraph `follows`, a backslash or two or more spaces
    /// that end it make a hard line break.
    fn of(source: &str, end: usize, follows: bool) -> Self {
        match line_end(source, end).filter(|_| follows) {
            Some(close) if close == end => Self::Backslash,
            Some(close) if source[end..close].ends_with("  ") => Self::Spaces(close),
            Some(_) | None => Self::Soft,
        }
    }
}

impl<'src> Layout<'src> {
    /// The content of a code span, at `range` of `text`, with the spaces and
    /// tabs before each line ending it crosses.
    fn code(&self, range: Range<usize>) -> String {
        let first = self.line(range.start);
        let mut value = String::new();
        for (index, part) in self.text[range].split('\n').enumerate() {
            if index > 0 {
                value.push_str(&self.source[self.trailing(first + index - 1)]);
                value.push('\n');
            }
            value.push_str(part);
        }
        value
    }

    /// Ends each line that a code span among `nodes` goes on from past the
    /// spaces and tabs before its line ending, which the code span holds.
    fn code_ends<E>(&mut self, nodes: &[Inline<E>])
    where
        E: Extension,
    {
        for node in nodes {
            match node {
                Inline::Code(code) => {
                    let lines = self.lines_of(&code.span);
                    for line in *lines.start()..*lines.end() {
                        self.endings[line] = Ending::Code(self.trailing(line).end);
                    }
                }
                Inline::Emphasis(emphasis) => self.code_ends(&emphasis.children),
                Inline::Link(link) => self.code_ends(&link.children),
                Inline::Strong(strong) => self.code_ends(&strong.children),
                Inline::Html(_) | Inline::Image(_) | Inline::LineBreak(_) | Inline::Text(_) => {}
            }
        }
    }

    /// The pieces of `node`, read from `text`, one for each line it lies
    /// on, by that line, with their spans in the source.
    fn cut<E>(&self, node: Inline<E>) -> Vec<(usize, Inline<E>)>
    where
        E: Extension,
    {
        let read = node.span().clone();
        match node {
            Inline::Code(code) => {
                let mut values = code.value.split('\n');
                self.pieces(&read, |continues, span| {
                    Inline::Code(Code {
                        continues,
                        kind: Kind(None),
                        span,
                        value: values.next().unwrap_or_default().to_owned(),
                    })
                })
            }
            Inline::Emphasis(emphasis) => {
                let mut children = self.spread(emphasis.children, &read);
                self.pieces(&read, |continues, span| {
                    Inline::Emphasis(Emphasis {
                        children: children.next().unwrap_or_default(),
                        continues,
                        kind: Kind(None),
                        span,
                    })
                })
            }
            Inline::Html(html) => {
                let mut values = html.value.split('\n');
                self.pieces(&read, |continues, span| {
                    Inline::Html(Html {
                        continues,
                        kind: Kind(None),
                        span,
                        value: values.next().unwrap_or_default().to_owned(),
                    })
                })
            }
            Inline::Image(image) => {
                let mut alts = image.alt.split('\n');
                self.pieces(&read, |continues, span| {
                    Inline::Image(Image {
                        alt: alts.next().unwrap_or_default().to_owned(),
                        continues,
                        form: image.form,
                        kind: Kind(None),
                        span,
                        target: image.target.clone(),
                        title: image.title.clone(),
                    })
                })
            }
            Inline::LineBreak(mut line_break) => {
                let line = self.line(read.start);
                // A break of spaces sits at its line end with an empty span.
                line_break.span = if read.is_empty() {
                    self.ranges[line].end..self.end(line)
                } else {
                    self.place(read.start)..self.place(read.end)
                };
                vec![(line, Inline::LineBreak(line_break))]
            }
            Inline::Link(link) => {
                let mut children = self.spread(link.children, &read);
                self.pieces(&read, |continues, span| {
                    Inline::Link(Link {
                        children: children.next().unwrap_or_default(),
                        continues,
                        form: link.form,
                        kind: Kind(None),
                        span,
                        target: link.target.clone(),
                        title: link.title.clone(),
                    })
                })
            }
            Inline::Strong(strong) => {
                let mut children = self.spread(strong.children, &read);
                self.pieces(&read, |continues, span| {
                    Inline::Strong(Strong {
                        children: children.next().unwrap_or_default(),
                        continues,
                        kind: Kind(None),
                        span,
                    })
                })
            }
            Inline::Text(mut text) => {
                text.span = self.place(read.start)..self.place(read.end);
                vec![(self.line(read.start), Inline::Text(text))]
            }
        }
    }

    /// Where the `Line` of `line` ends in the source: past the spaces of a
    /// hard line break that ends it, or of a code span that goes on from it,
    /// or at the end of its text.
    fn end(&self, line: usize) -> usize {
        match self.endings[line] {
            Ending::Code(end) | Ending::Spaces(end) => end,
            Ending::Backslash | Ending::Soft => self.ranges[line].end,
        }
    }

    /// How the line that holds `at`, a place in `text`, ends.
    fn ending(&self, at: usize) -> Ending {
        self.endings[self.line(at)]
    }

    /// The line that holds `at`, a place in `text`. The `\n` after a line
    /// belongs to it.
    fn line(&self, at: usize) -> usize {
        self.places
            .partition_point(|place| place.start <= at)
            .saturating_sub(1)
    }

    /// The lines of the layout, each with its pieces of `nodes`.
    fn lines<E>(&self, nodes: Vec<Inline<E>>) -> Vec<Line<E>>
    where
        E: Extension,
    {
        self.spread(nodes, &(0..self.text.len()))
            .zip(self.ranges)
            .enumerate()
            .map(|(line, (children, range))| Line {
                children,
                kind: Kind(None),
                span: range.start..self.end(line),
            })
            .collect()
    }

    /// The first and the last line that `span`, in `text`, lies on.
    fn lines_of(&self, span: &Range<usize>) -> RangeInclusive<usize> {
        let last = span.end.saturating_sub(1).max(span.start);
        self.line(span.start)..=self.line(last)
    }

    fn new(source: &'src str, ranges: &'src [Range<usize>]) -> Self {
        let mut text = String::new();
        let mut places = Vec::with_capacity(ranges.len());
        let mut endings = Vec::with_capacity(ranges.len());
        for (index, range) in ranges.iter().enumerate() {
            if index > 0 {
                text.push('\n');
            }
            let start = text.len();
            text.push_str(&source[range.clone()]);
            places.push(start..text.len());
            endings.push(Ending::of(source, range.end, index + 1 < ranges.len()));
        }
        Self {
            endings,
            places,
            ranges,
            source,
            text,
        }
    }

    /// The pieces of the node read at `read` in `text`, one for each line it
    /// lies on, by that line. `build` makes each piece from how it goes on
    /// and its span in the source: from the node's start, or the line's, to
    /// the node's end, or the line's.
    fn pieces<E, F>(&self, read: &Range<usize>, mut build: F) -> Vec<(usize, Inline<E>)>
    where
        E: Extension,
        F: FnMut(Continues, Range<usize>) -> Inline<E>,
    {
        let lines = self.lines_of(read);
        let (first, last) = (*lines.start(), *lines.end());
        lines
            .map(|line| {
                let start = if line == first {
                    self.place(read.start)
                } else {
                    self.ranges[line].start
                };
                let end = if line == last {
                    self.place(read.end)
                } else {
                    self.end(line)
                };
                let continues = match (line > first, line < last) {
                    (false, false) => Continues::Neither,
                    (false, true) => Continues::Next,
                    (true, false) => Continues::Previous,
                    (true, true) => Continues::Both,
                };
                (line, build(continues, start..end))
            })
            .collect()
    }

    /// The place in the source of `at`, a place in `text`.
    fn place(&self, at: usize) -> usize {
        let line = self.line(at);
        self.ranges[line].start + (at - self.places[line].start)
    }

    /// The pieces of `nodes`, the children of a node read at `read` in
    /// `text`, on each line that node lies on, in order.
    fn spread<E>(
        &self,
        nodes: Vec<Inline<E>>,
        read: &Range<usize>,
    ) -> impl Iterator<Item = Vec<Inline<E>>>
    where
        E: Extension,
    {
        let lines = self.lines_of(read);
        let first = *lines.start();
        let mut spread: Vec<Vec<Inline<E>>> = lines.map(|_| Vec::new()).collect();
        for node in nodes {
            for (line, piece) in self.cut(node) {
                spread[line - first].push(piece);
            }
        }
        spread.into_iter()
    }

    /// The spaces and tabs between the text of `line` and its line ending,
    /// in the source: none when something else follows them.
    fn trailing(&self, line: usize) -> Range<usize> {
        let end = self.ranges[line].end;
        end..line_end(self.source, end).unwrap_or(end)
    }
}

impl Run {
    /// Which openers a closer can pair with: by its character, whether it
    /// can also open, and its length modulo 3.
    fn key(&self) -> usize {
        usize::from(self.character == '_') * 6 + usize::from(self.can_open) * 3 + self.length % 3
    }

    /// Whether this run can open the emphasis `closer` closes. When either
    /// run can both open and close, their lengths may not add up to a
    /// multiple of 3 unless both are multiples of 3.
    const fn opens(&self, closer: &Self) -> bool {
        let both = self.length + closer.length;
        self.can_open
            && self.character == closer.character
            && (!(self.can_close || closer.can_open)
                || !both.is_multiple_of(3)
                || (self.length.is_multiple_of(3) && closer.length.is_multiple_of(3)))
    }

    /// The characters of the run not yet paired, as text.
    fn text<E>(&self) -> Inline<E>
    where
        E: Extension,
    {
        Inline::Text(Text {
            kind: Kind(None),
            span: self.span.clone(),
            value: self.character.to_string().repeat(self.count),
        })
    }
}

impl<'lay, 'defs, E> Scanner<'lay, 'defs, E>
where
    E: Extension,
{
    /// An autolink, inline HTML, or a `<` that starts neither.
    fn angle(&mut self, at: usize, end: usize) -> usize {
        let source = self.layout.text.as_str();
        if let Some((target, close)) = autolink_at(source, at, end) {
            let shown = at + 1..close - 1;
            self.push(Inline::Link(Link {
                children: vec![Inline::Text(Text {
                    kind: Kind(None),
                    value: source[shown.clone()].to_owned(),
                    span: shown,
                })],
                continues: Continues::Neither,
                form: LinkForm::Markdown,
                kind: Kind(None),
                span: at..close,
                target,
                title: None,
            }));
            return close;
        }
        if let Some(close) = html::inline_end(&source[..end], at) {
            self.push(Inline::Html(Html {
                continues: Continues::Neither,
                kind: Kind(None),
                span: at..close,
                value: source[at..close].to_owned(),
            }));
            return close;
        }
        self.push_text(at..at + 1, "<");
        at + 1
    }

    /// An image, or a `!` that starts none.
    fn bang(&mut self, at: usize, end: usize) -> usize {
        if let Some((image, close)) = image_at(&self.layout.text, self.definitions, at, end) {
            self.push(Inline::Image(image));
            return close;
        }
        self.push_text(at..at + 1, "!");
        at + 1
    }

    /// A link, or a `[` that starts none.
    fn bracket(&mut self, at: usize, end: usize) -> usize {
        if let Some(close) = self.link(at, end) {
            return close;
        }
        self.push_text(at..at + 1, "[");
        at + 1
    }

    /// A code span closes at the next run of exactly as many backticks.
    fn code(&mut self, at: usize, end: usize) -> usize {
        let source = self.layout.text.as_str();
        let ticks = run(source.as_bytes(), at, end);
        let Some(close) = code_end(source.as_bytes(), at, end) else {
            self.push_text(at..at + ticks, &source[at..at + ticks]);
            return at + ticks;
        };
        let content = self.layout.code(at + ticks..close - ticks);
        self.push(Inline::Code(Code {
            continues: Continues::Neither,
            kind: Kind(None),
            span: at..close,
            value: unpad(&content).to_owned(),
        }));
        close
    }

    /// A run of `*` or `_`, which can open emphasis when it is
    /// left-flanking and close it when it is right-flanking, by the
    /// characters around it. A `_` inside a word does neither.
    fn delimiters(&mut self, at: usize, end: usize) -> usize {
        let source = self.layout.text.as_str();
        let character = char::from(source.as_bytes()[at]);
        let length = source[at..end]
            .chars()
            .take_while(|&next| next == character)
            .count();
        let close = at + length;
        let before = source[..at].chars().next_back();
        let after = source[close..].chars().next();
        let space_before = before.is_none_or(is_whitespace);
        let space_after = after.is_none_or(is_whitespace);
        let mark_before = before.is_some_and(is_punctuation);
        let mark_after = after.is_some_and(is_punctuation);
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
        self.flush();
        self.pieces.push(Piece::Run(self.runs.len()));
        self.runs.push(Run {
            can_close,
            can_open,
            character,
            count: length,
            length,
            span: at..close,
        });
        close
    }

    /// The end of a line: the text read so far ends, and two or more spaces
    /// that end the line are a hard line break. The text does not hold those
    /// spaces, so the break has an empty span at the line end.
    fn end_line(&mut self, at: usize) -> usize {
        if matches!(self.layout.ending(at), Ending::Spaces(_)) {
            self.push(Inline::LineBreak(LineBreak {
                kind: Kind(None),
                span: at..at,
            }));
        } else {
            self.flush();
        }
        at + 1
    }

    /// An entity or numeric character reference, as the characters it
    /// names, or a `&` that starts none.
    fn entity(&mut self, at: usize, end: usize) -> usize {
        let source = self.layout.text.as_str();
        if let Some((characters, length)) = entity::reference(&source[at..end]) {
            self.push_text(at..at + length, &characters);
            return at + length;
        }
        self.push_text(at..at + 1, "&");
        at + 1
    }

    /// A backslash before ASCII punctuation makes that character plain text.
    /// A backslash that ends a line followed by another is a hard line break.
    fn escape(&mut self, at: usize, end: usize) -> usize {
        let source = self.layout.text.as_str();
        let bytes = source.as_bytes();
        if bytes.get(at + 1) == Some(&b'\n') && matches!(self.layout.ending(at), Ending::Backslash)
        {
            self.push(Inline::LineBreak(LineBreak {
                kind: Kind(None),
                span: at..at + 1,
            }));
            return at + 1;
        }
        let escaped = at + 1 < end && bytes[at + 1].is_ascii_punctuation();
        let (from, to) = if escaped {
            (at + 1, at + 2)
        } else {
            (at, at + 1)
        };
        self.push_text(at..to, &source[from..to]);
        to
    }

    /// The nodes read, with their runs paired into emphasis.
    fn finish(mut self) -> Vec<Inline<E>> {
        self.flush();
        emphasize(self.pieces, self.runs)
    }

    fn flush(&mut self) {
        if let Some(start) = self.text_start.take() {
            self.pieces.push(Piece::Node(Inline::Text(Text {
                kind: Kind(None),
                span: start..self.text_end,
                value: mem::take(&mut self.text),
            })));
        }
    }

    /// A scanner for the text of a link, which takes over the holders found
    /// so far.
    fn inner(&mut self) -> Self {
        Self {
            definitions: self.definitions,
            holders: mem::take(&mut self.holders),
            layout: self.layout,
            linked: false,
            pieces: Vec::new(),
            runs: Vec::new(),
            text: String::new(),
            text_end: 0,
            text_start: None,
        }
    }

    /// A wiki link, `[text](destination "title")`, or a reference link.
    /// Brackets whose text holds a link make none.
    fn link(&mut self, at: usize, end: usize) -> Option<usize> {
        if self.holders.contains(&at) {
            return None;
        }
        let source = self.layout.text.as_str();
        if let Some(wiki) = wiki_at(source, at, end) {
            let children = wiki.alias.map_or_else(Vec::new, |alias| {
                vec![Inline::Text(Text {
                    kind: Kind(None),
                    value: source[alias.clone()].to_owned(),
                    span: alias,
                })]
            });
            self.linked = true;
            self.push(Inline::Link(Link {
                children,
                continues: Continues::Neither,
                form: LinkForm::Wiki,
                kind: Kind(None),
                span: at..wiki.close,
                target: source[wiki.target].to_owned(),
                title: None,
            }));
            return Some(wiki.close);
        }
        let text_close = text_end(source, at, end)?;
        let (target, close) = definition::inline(&source[..end], text_close + 1)
            .or_else(|| reference(source, self.definitions, at, text_close, end))?;
        let mut inner = self.inner();
        inner.scan(at + 1..text_close);
        self.holders = mem::take(&mut inner.holders);
        if inner.linked {
            self.holders.insert(at);
            return None;
        }
        self.linked = true;
        self.push(Inline::Link(Link {
            children: inner.finish(),
            continues: Continues::Neither,
            form: LinkForm::Markdown,
            kind: Kind(None),
            span: at..close,
            target: target.destination,
            title: target.title,
        }));
        Some(close)
    }

    fn new(layout: &'lay Layout<'lay>, definitions: &'defs Definitions) -> Self {
        Self {
            definitions,
            holders: HashSet::new(),
            layout,
            linked: false,
            pieces: Vec::new(),
            runs: Vec::new(),
            text: String::new(),
            text_end: 0,
            text_start: None,
        }
    }

    /// Text up to the next character that can start another node, or the
    /// end of the line.
    fn plain(&mut self, at: usize, end: usize) -> usize {
        let source = self.layout.text.as_str();
        let bytes = source.as_bytes();
        let mut next = at + 1;
        while next < end
            && !matches!(
                bytes[next],
                b'\n' | b'\\' | b'`' | b'[' | b'!' | b'&' | b'<' | b'*' | b'_'
            )
        {
            next += 1;
        }
        self.push_text(at..next, &source[at..next]);
        next
    }

    /// Ends the text read so far and adds `node` after it.
    fn push(&mut self, node: Inline<E>) {
        self.flush();
        self.pieces.push(Piece::Node(node));
    }

    fn push_text(&mut self, range: Range<usize>, value: &str) {
        if self.text_start.is_none() {
            self.text_start = Some(range.start);
        }
        self.text_end = range.end;
        self.text.push_str(value);
    }

    fn scan(&mut self, range: Range<usize>) {
        let bytes = self.layout.text.as_bytes();
        let mut at = range.start;
        while at < range.end {
            at = match bytes[at] {
                b'\n' => self.end_line(at),
                b'\\' => self.escape(at, range.end),
                b'`' => self.code(at, range.end),
                b'!' => self.bang(at, range.end),
                b'&' => self.entity(at, range.end),
                b'<' => self.angle(at, range.end),
                b'*' | b'_' => self.delimiters(at, range.end),
                b'[' => self.bracket(at, range.end),
                _ => self.plain(at, range.end),
            };
        }
    }
}

/// The lines of a paragraph and the inline nodes written on them, with the
/// link `definitions` of the file. Nodes are matched over the whole
/// paragraph, and a node that crosses a line end is cut into one piece per
/// line.
pub fn lines<E>(source: &str, ranges: &[Range<usize>], definitions: &Definitions) -> Vec<Line<E>>
where
    E: Extension,
{
    let mut layout = Layout::new(source, ranges);
    let mut scanner = Scanner::new(&layout, definitions);
    scanner.scan(0..layout.text.len());
    let nodes = scanner.finish();
    layout.code_ends(&nodes);
    layout.lines(nodes)
}

/// Pairs the runs among `pieces` into emphasis and strong emphasis, by
/// `CommonMark`'s procedure for processing emphasis, and gives the nodes.
fn emphasize<E>(mut pieces: Vec<Piece<E>>, mut runs: Vec<Run>) -> Vec<Inline<E>>
where
    E: Extension,
{
    // For each key of closer, the first run that may still open for it.
    let mut bottoms = [0_usize; 12];
    let mut current = 0;
    while let Some(piece) = pieces.get(current) {
        let closer = match piece {
            Piece::Run(closer) if runs[*closer].can_close => *closer,
            Piece::Node(_) | Piece::Run(_) => {
                current += 1;
                continue;
            }
        };
        let key = runs[closer].key();
        let opener = pieces[..current]
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(index, candidate)| match candidate {
                Piece::Run(run) => Some((index, *run)),
                Piece::Node(_) => None,
            })
            .take_while(|&(_, run)| run >= bottoms[key])
            .find(|&(_, run)| runs[run].opens(&runs[closer]));
        if let Some(open) = opener {
            current = pair(&mut pieces, &mut runs, open, (current, closer));
        } else {
            bottoms[key] = closer;
            if !runs[closer].can_open {
                pieces[current] = Piece::Node(runs[closer].text());
            }
            current += 1;
        }
    }
    nodes(pieces, &runs)
}

/// Makes emphasis, or strong emphasis when both runs have two characters
/// left, of the pieces between the opening and the closing run, each given
/// by its place among `pieces` and among `runs`. Gives where the closer, or
/// what follows it, now is.
fn pair<E>(
    pieces: &mut Vec<Piece<E>>,
    runs: &mut [Run],
    (open, opener): (usize, usize),
    (close, closer): (usize, usize),
) -> usize
where
    E: Extension,
{
    let used = if runs[opener].count >= 2 && runs[closer].count >= 2 {
        2
    } else {
        1
    };
    runs[opener].count -= used;
    runs[opener].span.end -= used;
    runs[closer].count -= used;
    runs[closer].span.start += used;
    let children = nodes(pieces.drain(open + 1..close), runs);
    let span = runs[opener].span.end..runs[closer].span.start;
    let node = if used == 2 {
        Inline::Strong(Strong {
            children,
            continues: Continues::Neither,
            kind: Kind(None),
            span,
        })
    } else {
        Inline::Emphasis(Emphasis {
            children,
            continues: Continues::Neither,
            kind: Kind(None),
            span,
        })
    };
    pieces.insert(open + 1, Piece::Node(node));
    let mut after = open + 2;
    if runs[opener].count == 0 {
        pieces.remove(open);
        after -= 1;
    }
    if runs[closer].count == 0 {
        pieces.remove(after);
    }
    after
}

/// `pieces` as nodes: each run as the text of its characters not paired,
/// and texts that touch joined into one. A line end lies between the texts
/// of two lines.
fn nodes<E, I>(pieces: I, runs: &[Run]) -> Vec<Inline<E>>
where
    E: Extension,
    I: IntoIterator<Item = Piece<E>>,
{
    let mut nodes: Vec<Inline<E>> = Vec::new();
    for piece in pieces {
        let node = match piece {
            Piece::Node(node) => node,
            Piece::Run(run) => runs[run].text(),
        };
        match (nodes.last_mut(), node) {
            (Some(Inline::Text(last)), Inline::Text(text)) if last.span.end == text.span.start => {
                last.value.push_str(&text.value);
                last.span.end = text.span.end;
            }
            (_, other) => nodes.push(other),
        }
    }
    nodes
}

/// A Unicode whitespace character, as `CommonMark` counts them: a space
/// separator, a tab, a line feed, a form feed or a carriage return.
pub fn is_whitespace(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\u{c}' | '\r')
        || character.general_category() == GeneralCategory::SpaceSeparator
}

/// A Unicode punctuation character, as `CommonMark` counts them: one of the
/// punctuation or symbol general categories.
pub fn is_punctuation(character: char) -> bool {
    matches!(
        character.general_category_group(),
        GeneralCategoryGroup::Punctuation | GeneralCategoryGroup::Symbol
    )
}

/// The target of the autolink at `at`, `<scheme:rest>` or `<address>`, and
/// where it ends, past its `>`.
fn autolink_at(source: &str, at: usize, end: usize) -> Option<(String, usize)> {
    let rest = &source[at + 1..end];
    let inner = &rest[..rest.find('>')?];
    let close = at + inner.len() + 2;
    if is_uri(inner) {
        return Some((inner.to_owned(), close));
    }
    is_email(inner).then(|| (format!("mailto:{inner}"), close))
}

/// A scheme of 2 to 32 letters, digits, `+`, `.` or `-` that starts with a
/// letter, then `:`, then no space, control character or `<`.
fn is_uri(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once(':') else {
        return false;
    };
    (2..=32).contains(&scheme.len())
        && scheme.starts_with(|letter: char| letter.is_ascii_alphabetic())
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
        && !rest
            .chars()
            .any(|character| character.is_ascii_control() || matches!(character, ' ' | '<'))
}

/// An email address as `CommonMark` reads one in an autolink.
fn is_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&byte))
        && domain.split('.').all(|label| {
            (1..=63).contains(&label.len())
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

/// `[[target]]` or `[[target|alias]]` starting at `at`, on one line. Neither
/// part holds a bracket.
fn wiki_at(source: &str, at: usize, end: usize) -> Option<Wiki> {
    let inner = source[at..end].strip_prefix("[[")?;
    let length = inner.find("]]")?;
    let written = &inner[..length];
    if written.is_empty() || written.contains(['[', ']', '\n']) {
        return None;
    }
    let start = at + 2;
    let (target, alias) = written
        .find('|')
        .map_or((start..start + length, None), |bar| {
            (start..start + bar, Some(start + bar + 1..start + length))
        });
    Some(Wiki {
        alias: alias.filter(|range| !range.is_empty()),
        close: start + length + 2,
        target,
    })
}

/// The `]` that ends the text of a link or an image whose `[` is at
/// `open`: the first that closes every bracket opened after it. A bracket
/// after a backslash, or inside a code span, an autolink, inline HTML, a
/// wiki embed or the destination and title of an image, does not count.
fn text_end(source: &str, open: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open) != Some(&b'[') {
        return None;
    }
    // Whether each bracket still open opens an image.
    let mut brackets = Vec::new();
    let mut at = open + 1;
    while at < end {
        at = match bytes[at] {
            b']' => match brackets.pop() {
                None => return Some(at),
                Some(true) => {
                    definition::inline(&source[..end], at + 1).map_or(at + 1, |(_, close)| close)
                }
                Some(false) => at + 1,
            },
            b'[' => {
                brackets.push(false);
                at + 1
            }
            b'!' if bytes.get(at + 1) == Some(&b'[') => wiki_at(source, at + 1, end).map_or_else(
                || {
                    brackets.push(true);
                    at + 2
                },
                |wiki| wiki.close,
            ),
            b'\\' if at + 1 < end && bytes[at + 1].is_ascii_punctuation() => at + 2,
            b'`' => code_end(bytes, at, end).unwrap_or_else(|| at + run(bytes, at, end)),
            b'<' => autolink_at(source, at, end)
                .map(|(_, close)| close)
                .or_else(|| html::inline_end(&source[..end], at))
                .unwrap_or(at + 1),
            _ => at + 1,
        };
    }
    None
}

/// An image starting at the `!` at `at`, and where it ends: a wiki embed,
/// `![alt](destination "title")`, or a reference image, which takes the
/// target of the definition its label names.
fn image_at<E>(
    source: &str,
    definitions: &Definitions,
    at: usize,
    end: usize,
) -> Option<(Image<E>, usize)>
where
    E: Extension,
{
    let open = at + 1;
    if let Some(wiki) = wiki_at(source, open, end) {
        let image = Image {
            alt: wiki
                .alias
                .map_or_else(String::new, |alias| source[alias].to_owned()),
            continues: Continues::Neither,
            form: LinkForm::Wiki,
            kind: Kind(None),
            span: at..wiki.close,
            target: source[wiki.target].to_owned(),
            title: None,
        };
        return Some((image, wiki.close));
    }
    let text_close = text_end(source, open, end)?;
    let (target, close) = definition::inline(&source[..end], text_close + 1)
        .or_else(|| reference(source, definitions, open, text_close, end))?;
    let image = Image {
        alt: source[open + 1..text_close].to_owned(),
        continues: Continues::Neither,
        form: LinkForm::Markdown,
        kind: Kind(None),
        span: at..close,
        target: target.destination,
        title: target.title,
    };
    Some((image, close))
}

/// The target of the reference link or image whose text runs from the `[`
/// at `open` to the `]` at `close` in `source`, read up to `end`, and where
/// it ends: `[text][label]`, `[label][]` or `[label]`. A label with no
/// definition gives none.
fn reference(
    source: &str,
    definitions: &Definitions,
    open: usize,
    close: usize,
    end: usize,
) -> Option<(Target, usize)> {
    let text = &source[..end];
    let named = |label: &str| definitions.get(&fold_label(label)).cloned();
    let after = close + 1;
    if let Some(label_close) = definition::label_at(text, after) {
        let target = named(&text[after + 1..label_close])?;
        return Some((target, label_close + 1));
    }
    let label_close = definition::label_at(text, open).filter(|&found| found == close)?;
    let target = named(&text[open + 1..label_close])?;
    let link_end = if text[after..].starts_with("[]") {
        after + 2
    } else {
        after
    };
    Some((target, link_end))
}

/// The content of a code span without one space at each end, when both
/// ends are spaces and it is not all spaces, a line ending counting as one.
fn unpad(content: &str) -> &str {
    if content.bytes().all(|byte| matches!(byte, b' ' | b'\n')) {
        return content;
    }
    content
        .strip_prefix(' ')
        .and_then(|rest| rest.strip_suffix(' '))
        .unwrap_or(content)
}

/// Past the spaces and tabs at `end` in `source`, when a line ending, or
/// the end of `source`, follows them.
fn line_end(source: &str, end: usize) -> Option<usize> {
    let close = end
        + source[end..]
            .bytes()
            .take_while(|&byte| matches!(byte, b' ' | b'\t'))
            .count();
    matches!(source.as_bytes().get(close), None | Some(b'\n' | b'\r')).then_some(close)
}

/// Where the code span whose opening backticks are at `at` ends, past its
/// closing run of exactly as many backticks.
fn code_end(bytes: &[u8], at: usize, end: usize) -> Option<usize> {
    let ticks = run(bytes, at, end);
    let mut close = at + ticks;
    while close < end {
        if bytes[close] != b'`' {
            close += 1;
            continue;
        }
        let found = run(bytes, close, end);
        if found == ticks {
            return Some(close + ticks);
        }
        close += found;
    }
    None
}

/// The length of the run of backticks starting at `at`.
fn run(bytes: &[u8], at: usize, end: usize) -> usize {
    bytes[at..end]
        .iter()
        .take_while(|&&byte| byte == b'`')
        .count()
}
