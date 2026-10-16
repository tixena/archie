//! Inline nodes inside the text lines of a paragraph or a heading.

use core::mem;
use core::ops::Range;

use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory as _};

use crate::parser::ast::{
    Code, Emphasis, Html, Image, Inline, Kind, Line, LineBreak, Link, LinkForm, Strong, Text,
};
use crate::parser::definition::{self, Definitions, Target, fold_label};
use crate::parser::extension::{Extension, Plain};
use crate::parser::html;

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

struct Scanner<'src, 'defs, E>
where
    E: Extension,
{
    /// The line read: its ends count as whitespace beside a run.
    bounds: Range<usize>,
    /// Set when a backslash that ends the scanned range is a hard line break.
    breaks: bool,
    definitions: &'defs Definitions,
    /// Off inside the text of a link, where another link cannot start.
    links: bool,
    pieces: Vec<Piece<E>>,
    runs: Vec<Run>,
    source: &'src str,
    text: String,
    text_end: usize,
    text_start: Option<usize>,
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

impl<'src, 'defs, E> Scanner<'src, 'defs, E>
where
    E: Extension,
{
    /// An autolink, inline HTML, or a `<` that starts neither.
    fn angle(&mut self, at: usize, end: usize) -> usize {
        let source = self.source;
        if let Some((target, close)) = autolink_at(source, at, end) {
            let shown = at + 1..close - 1;
            self.push(Inline::Link(Link {
                children: vec![Inline::Text(Text {
                    kind: Kind(None),
                    value: source[shown.clone()].to_owned(),
                    span: shown,
                })],
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
        if let Some((image, close)) = image_at(self.source, at, end) {
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
        let source = self.source;
        let ticks = run(source.as_bytes(), at, end);
        let Some(close) = code_end(source.as_bytes(), at, end) else {
            self.push_text(at..at + ticks, &source[at..at + ticks]);
            return at + ticks;
        };
        self.push(Inline::Code(Code {
            kind: Kind(None),
            span: at..close,
            value: source[at + ticks..close - ticks].to_owned(),
        }));
        close
    }

    /// A run of `*` or `_`, which can open emphasis when it is
    /// left-flanking and close it when it is right-flanking, by the
    /// characters around it. A `_` inside a word does neither.
    fn delimiters(&mut self, at: usize, end: usize) -> usize {
        let source = self.source;
        let character = char::from(source.as_bytes()[at]);
        let length = source[at..end]
            .chars()
            .take_while(|&next| next == character)
            .count();
        let close = at + length;
        let before = source[self.bounds.start..at].chars().next_back();
        let after = source[close..self.bounds.end].chars().next();
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

    /// A backslash before ASCII punctuation makes that character plain text.
    /// A backslash that ends a line followed by another is a hard line break.
    fn escape(&mut self, at: usize, end: usize) -> usize {
        if self.breaks && at + 1 == end {
            self.push(Inline::LineBreak(LineBreak {
                kind: Kind(None),
                span: at..end,
            }));
            return end;
        }
        let escaped = at + 1 < end && self.source.as_bytes()[at + 1].is_ascii_punctuation();
        let (from, to) = if escaped {
            (at + 1, at + 2)
        } else {
            (at, at + 1)
        };
        let source = self.source;
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

    /// A scanner for the text of a link on the same line.
    const fn inner(&self) -> Self {
        Self {
            bounds: self.bounds.start..self.bounds.end,
            breaks: false,
            definitions: self.definitions,
            links: false,
            pieces: Vec::new(),
            runs: Vec::new(),
            source: self.source,
            text: String::new(),
            text_end: 0,
            text_start: None,
        }
    }

    /// A wiki link, `[text](destination "title")`, or a reference link.
    fn link(&mut self, at: usize, end: usize) -> Option<usize> {
        let source = self.source;
        if let Some(wiki) = wiki_at(source, at, end) {
            let children = wiki.alias.map_or_else(Vec::new, |alias| {
                vec![Inline::Text(Text {
                    kind: Kind(None),
                    value: source[alias.clone()].to_owned(),
                    span: alias,
                })]
            });
            self.push(Inline::Link(Link {
                children,
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
            .or_else(|| self.reference(at, text_close, end))?;
        let mut inner = self.inner();
        inner.scan(at + 1..text_close);
        self.push(Inline::Link(Link {
            children: inner.finish(),
            form: LinkForm::Markdown,
            kind: Kind(None),
            span: at..close,
            target: target.destination,
            title: target.title,
        }));
        Some(close)
    }

    const fn new(
        source: &'src str,
        bounds: Range<usize>,
        definitions: &'defs Definitions,
        breaks: bool,
    ) -> Self {
        Self {
            bounds,
            breaks,
            definitions,
            links: true,
            pieces: Vec::new(),
            runs: Vec::new(),
            source,
            text: String::new(),
            text_end: 0,
            text_start: None,
        }
    }

    /// Text up to the next character that can start another node.
    fn plain(&mut self, at: usize, end: usize) -> usize {
        let bytes = self.source.as_bytes();
        let mut next = at + 1;
        while next < end && !matches!(bytes[next], b'\\' | b'`' | b'[' | b'!' | b'<' | b'*' | b'_')
        {
            next += 1;
        }
        let source = self.source;
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

    /// The target of the reference link whose text runs from the `[` at
    /// `open` to the `]` at `close`, and where the link ends:
    /// `[text][label]`, `[label][]` or `[label]`. A label with no
    /// definition gives none.
    fn reference(&self, open: usize, close: usize, end: usize) -> Option<(Target, usize)> {
        let text = &self.source[..end];
        let after = close + 1;
        if let Some(label_close) = definition::label_at(text, after) {
            let target = self.target(&text[after + 1..label_close])?;
            return Some((target, label_close + 1));
        }
        let label_close = definition::label_at(text, open).filter(|&found| found == close)?;
        let target = self.target(&text[open + 1..label_close])?;
        let link_end = if text[after..].starts_with("[]") {
            after + 2
        } else {
            after
        };
        Some((target, link_end))
    }

    fn scan(&mut self, range: Range<usize>) {
        let bytes = self.source.as_bytes();
        let mut at = range.start;
        while at < range.end {
            at = match bytes[at] {
                b'\\' => self.escape(at, range.end),
                b'`' => self.code(at, range.end),
                b'!' => self.bang(at, range.end),
                b'<' => self.angle(at, range.end),
                b'*' | b'_' => self.delimiters(at, range.end),
                b'[' if self.links => self.bracket(at, range.end),
                _ => self.plain(at, range.end),
            };
        }
    }

    /// The definition `label` names.
    fn target(&self, label: &str) -> Option<Target> {
        self.definitions.get(&fold_label(label)).cloned()
    }
}

/// The inline nodes written on one line, with the link `definitions` of
/// the file. When another line of the paragraph `follows`, a backslash or
/// two or more spaces that end the line make a hard line break.
pub fn line<E>(
    source: &str,
    range: Range<usize>,
    definitions: &Definitions,
    follows: bool,
) -> Line<E>
where
    E: Extension,
{
    let trailing = source[range.end..]
        .bytes()
        .take_while(|&byte| matches!(byte, b' ' | b'\t'))
        .count();
    let ending = range.end + trailing;
    let breaks = follows && matches!(source.as_bytes().get(ending), None | Some(b'\n' | b'\r'));
    let spaced = breaks && source[range.end..ending].ends_with("  ");
    let mut scanner = Scanner::new(source, range.clone(), definitions, breaks && trailing == 0);
    scanner.scan(range.clone());
    let mut children = scanner.finish();
    let end = if spaced {
        children.push(Inline::LineBreak(LineBreak {
            kind: Kind(None),
            span: range.end..ending,
        }));
        ending
    } else {
        range.end
    };
    Line {
        children,
        kind: Kind(None),
        span: range.start..end,
    }
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
            kind: Kind(None),
            span,
        })
    } else {
        Inline::Emphasis(Emphasis {
            children,
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
/// and texts side by side joined into one.
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
            (Some(Inline::Text(last)), Inline::Text(text)) => {
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
fn is_whitespace(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\u{c}' | '\r')
        || character.general_category() == GeneralCategory::SpaceSeparator
}

/// A Unicode punctuation character, as `CommonMark` counts them: one of the
/// punctuation or symbol general categories.
fn is_punctuation(character: char) -> bool {
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

/// `[[target]]` or `[[target|alias]]` starting at `at`. Neither part holds a
/// bracket.
fn wiki_at(source: &str, at: usize, end: usize) -> Option<Wiki> {
    let inner = source[at..end].strip_prefix("[[")?;
    let length = inner.find("]]")?;
    let written = &inner[..length];
    if written.is_empty() || written.contains(['[', ']']) {
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
/// `open`. The text holds no bracket except inside an image, a code span,
/// an autolink or inline HTML, or after a backslash.
fn text_end(source: &str, open: usize, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(open) != Some(&b'[') {
        return None;
    }
    let mut at = open + 1;
    while at < end {
        at = match bytes[at] {
            b']' => return Some(at),
            b'[' => return None,
            b'\\' if at + 1 < end && bytes[at + 1].is_ascii_punctuation() => at + 2,
            b'`' => code_end(bytes, at, end).unwrap_or_else(|| at + run(bytes, at, end)),
            b'<' => autolink_at(source, at, end)
                .map(|(_, close)| close)
                .or_else(|| html::inline_end(&source[..end], at))
                .unwrap_or(at + 1),
            b'!' => image_at::<Plain>(source, at, end).map_or(at + 1, |(_, close)| close),
            _ => at + 1,
        };
    }
    None
}

/// An image starting at the `!` at `at`, and where it ends.
fn image_at<E>(source: &str, at: usize, end: usize) -> Option<(Image<E>, usize)>
where
    E: Extension,
{
    let open = at + 1;
    if let Some(wiki) = wiki_at(source, open, end) {
        let image = Image {
            alt: wiki
                .alias
                .map_or_else(String::new, |alias| source[alias].to_owned()),
            form: LinkForm::Wiki,
            kind: Kind(None),
            span: at..wiki.close,
            target: source[wiki.target].to_owned(),
            title: None,
        };
        return Some((image, wiki.close));
    }
    let text_close = text_end(source, open, end)?;
    let (target, close) = definition::inline(&source[..end], text_close + 1)?;
    let image = Image {
        alt: source[open + 1..text_close].to_owned(),
        form: LinkForm::Markdown,
        kind: Kind(None),
        span: at..close,
        target: target.destination,
        title: target.title,
    };
    Some((image, close))
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
