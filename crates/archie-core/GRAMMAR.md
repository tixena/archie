# The grammar of the markdown archie reads

This file states what `archie_core::parser::parse` and `parse_with` accept and the
tree they build, as the parser is written today. The unit tests under
[`src/parser/tests/`](src/parser/tests) show, sample by sample, the tree each rule
produces. None of the choices has been reviewed; the
[last section](#choices-made-without-a-requirement) lists the ones that were made
with no requirement behind them.

## Two stages

A grammar is context-free when each rule can be applied without looking at anything
outside the piece of text it matches. Three things in markdown and two in YAML break
that. Each one is taken out of the grammar and handled by the stage that reads
characters, so the grammar never sees it.

| What is not context-free | What handles it | What the grammar sees instead |
|---|---|---|
| Which list item a line belongs to depends on how far it is indented compared with earlier lines. | The body lexer keeps a stack of the open items. | `ITEM_START` and `ITEM_END` tokens. |
| A fenced code block ends at a fence at least as long as the one that opened it. | The body lexer reads the whole block. | One `CODE_BLOCK` token. |
| A table's cells line up in columns, and a pipe table's first line is a header only when the next line is a delimiter row. | The table reader reads the whole table. | One `TABLE` token. |
| A code span ends at a run of exactly as many backticks as opened it. | The inline scanner compares the two runs. | One `code` node. |
| Which YAML mapping a line belongs to depends on its indentation. | The YAML reader compares each line's indentation with that of the block it is reading. | `INDENT` and `DEDENT` in the rules below. |
| A YAML block scalar ends at the first line of text indented less than its own text. | The YAML reader reads the whole scalar. | One `BLOCK_SCALAR` token. |

Reference links (`[text][label]`, with `[label]: target` written elsewhere in the
file) are the other construct of CommonMark that is not context-free. archie does not
read them, so nothing has to handle them.

With those removed, every rule below is context-free.

| Stage | File |
|---|---|
| Finding the frontmatter | [`src/parser/frontmatter.rs`](src/parser/frontmatter.rs) |
| TOML frontmatter | [`src/parser/toml.rs`](src/parser/toml.rs) |
| YAML frontmatter | [`src/parser/yaml.rs`](src/parser/yaml.rs) |
| Typing an unquoted value | [`src/parser/scalar.rs`](src/parser/scalar.rs) |
| Body lexer | [`src/parser/lexer.rs`](src/parser/lexer.rs) |
| Tables | [`src/parser/table.rs`](src/parser/table.rs) |
| Body grammar | [`src/parser/block.rs`](src/parser/block.rs) |
| Inline grammar | [`src/parser/inline.rs`](src/parser/inline.rs) |
| The tree, and the `Extension` trait | [`src/parser/ast.rs`](src/parser/ast.rs) |

## Notation

```text
name := ...      a rule
A B              A followed by B
A | B            A or B
A*  A+  A?       zero or more, one or more, optional
UPPER            a token made by a lexer
"x"              the literal characters x
```

## Document

```text
document    := frontmatter? body
frontmatter := TOML_OPEN toml TOML_CLOSE
             | YAML_OPEN yaml YAML_CLOSE
```

- `TOML_OPEN` is the first line of the file when that line is `+++`. `TOML_CLOSE` is
  the next line that is `+++`.
- `YAML_OPEN` is the first line of the file when that line is `---`. `YAML_CLOSE` is
  the next line that is `---` or `...`.
- Spaces after a delimiter are allowed. Spaces before it are not.
- When the first line is a delimiter and no later line closes it, the file has no
  frontmatter and that first line is body text.
- The body starts on the line after the closing delimiter. Both `\n` and `\r\n` end
  a line.

Only the frontmatter can fail. The body never fails: what no rule recognizes is
text.

## Body

### Tokens

The lexer reads the body one line at a time. It keeps a stack with one number per
open list item: the column where that item's text starts.

1. A line holding only spaces and tabs is `BLANK`.
2. Otherwise the spaces that start the line are counted: that count is the line's
   column. Every open item whose text column is greater than the line's column is
   closed, innermost first, with one `ITEM_END` each.
3. What is written from that column on is classified by the first of these that
   fits:
   1. No item is open, and it is one to six `#` followed by a space or by the end
      of the line: `HEADING_n`, where `n` is the number of `#`.
   2. It starts with three or more backticks and holds no other backtick: a fenced
      code block starts. The lexer takes every line up to and including the first
      line made only of at least that many backticks, and emits one `CODE_BLOCK`
      holding the lines between and their range in the source. With no closing fence, the block ends before the
      first non-blank line indented less than the enclosing item, or at the end of
      the file.
   3. It is `-`, `*` or `+` followed by a space or by the end of the line, and the
      line is not three or more of the same `-`, `*` or `_` with only spaces
      between them: `ITEM_START`. The item's text column is the bullet's column
      plus two. Whatever follows the bullet on that line is classified again by
      these same rules.
   4. It starts a well-formed table (see [Tables](#tables)): one `TABLE` token
      holding the whole table. The lexer goes on after the table's last line.
   5. Anything else: `TEXT`, ending before any trailing spaces.
4. At the end of the file every open item is closed.

### Blocks

```text
body       := content_0
content_n  := plain* section_m*              for n = 0..6, every m greater than n
section_m  := HEADING_m content_m            for m = 1..6
plain      := BLANK | paragraph | list | CODE_BLOCK | TABLE
paragraph  := TEXT+
list       := item ( BLANK* item )*
item       := ITEM_START plain* ITEM_END
```

`content_n` and `section_m` stand for seven and six rules, one per level, so the
set of rules is finite. A section holds the blocks after its heading, then the
sections of deeper levels, and ends at the next heading of its own level or a
higher one.

Two rules settle what the grammar leaves open: a paragraph takes every `TEXT` that
follows it, and blank lines followed by an item continue the list.

### Tables

A table is read whole, and only when it is well formed; otherwise its lines are read
as they would be without it, which makes them paragraph text. Three formats are
read, tried in this order:

| Format | Written as | Well formed when |
|---|---|---|
| Pipe | A header row, then a delimiter row such as `\|:--\|:-:\|--:\|`, then body rows, each with cells between `\|`. The pipes at either end of a row are optional. | The delimiter row has one cell per header cell, each cell is dashes with an optional colon at either end, and every body row has that many cells. The table ends at the first line with no pipe, or a blank line. |
| Grid | `+---+---+` borders around rows of `\| a \| b \|`; a `+===+===+` border under the header rows. | Every line has the width of the top border, every `+` and `\|` stands at a column of the top border's `+`, and every row is one line between two borders. |
| Simple | A header line, then a line of dash groups separated by spaces, then rows up to a blank line. | There are at least two dash groups and one row, no text sits left of the first group, and no word crosses from one column into the next. A column runs from its dash group to the next group. |

- A cell holds one paragraph of inline nodes, or nothing when it is empty. `\|` in a
  pipe cell is a pipe, through the escape rule below.
- The alignment of a pipe column comes from the colons of its delimiter cell, and that
  of a grid column from the colons of the header border (or of the top border when
  there is no header): `:--` left, `--:` right, `:-:` center, `--` default.
- The alignment of a simple column comes from where its header sits on its dashes:
  flush with the left end only, left; with the right end only, right; with neither,
  center; with both, default.
- A pipe or simple table has one header row; a grid table has the rows above its
  `+===+` border, or none.

### Inline nodes

Inline nodes are read inside each `TEXT` line and inside the text of a heading.

```text
inlines     := inline*
inline      := code | image | link | escape | text
code        := TICKS code_text TICKS         both runs of backticks the same length
image       := "!" ( wiki | bracketed )
link        := wiki | bracketed
wiki        := "[[" target ( "|" alias )? "]]"
bracketed   := "[" link_inline* "]" "(" destination ")"
link_inline := code | image | escape | text
escape      := "\" PUNCTUATION
text        := any other characters
```

- A link and an image record which form they were written in: `wiki` or
  `bracketed`, called `Wiki` and `Markdown` in the tree.
- `target` and `alias` hold no `[` and no `]`. `target` holds no `|`. A link's
  alias is its text; an image's alias is its `alt`.
- `destination` holds no space, no tab and no `(`. It is the link's or the image's
  `target`.
- The text of a link holds no unescaped `[` or `]` other than an image's, so a link
  cannot hold a link but can hold an image. An image's `alt` is the text between
  its brackets, as written.
- `PUNCTUATION` is any ASCII punctuation character. The backslash is dropped and
  the character becomes text.
- An inline node starts and ends on one line.
- Brackets or backticks that complete none of these rules are text.

Neighbouring `text` and `escape` pieces become one `Text` node. Between two lines
of one paragraph, the line break is a `\n` inside that `Text`.

## TOML frontmatter

```text
toml         := ( pair | COMMENT | NEWLINE )*
pair         := key "=" value COMMENT? NEWLINE
key          := BARE_KEY | basic_string | literal_string
value        := basic_string | literal_string | BARE_VALUE | array | inline_table
array        := "[" ( value ( "," value )* ","? )? "]"
inline_table := "{" ( key "=" value ( "," key "=" value )* )? "}"
```

- `BARE_KEY` is letters, digits, `_` and `-`.
- `COMMENT` is `#` to the end of the line.
- Inside an array, line breaks and comments are allowed between the values. Inside
  an inline table they are not.
- A key written more than once in one mapping or inline table becomes one key, in
  the place where it was first written. Its value is a list of every value written
  for it, in the order of the file: a list adds its items, and any other value adds
  itself as one item. `title = "Draft"` then `title = "Final"` is
  `title = ["Draft", "Final"]`.

A value's type comes from how it is written:

| Written as | Type |
|---|---|
| `"..."`, with the escapes `\b \t \n \f \r \" \\ \uXXXX \UXXXXXXXX` | String |
| `'...'`, with no escapes | String |
| `true`, `false` | Boolean |
| `2026-10-10`, when that day exists in the calendar; `2026-02-30` is not a value | Date |
| `07:32:00`, optionally with a fraction such as `.250` | Time |
| A date, then `T`, `t` or one space, then a time, then optionally `Z`, `z` or an offset such as `-04:00` | DateTime |
| An optional sign and digits; or `0x`, `0o`, `0b` and digits in that base | Integer, 64 bits |
| Digits with a fraction, an exponent or both; `inf` and `nan`, with an optional sign | Float |

## YAML frontmatter

```text
yaml          := mapping?
mapping       := entry+
entry         := key ":" ( node | NEWLINE block? )
sequence      := ( "-" ( node | mapping | NEWLINE ) )+
node          := TAG? inline NEWLINE
               | TAG? BLOCK_SCALAR
               | TAG NEWLINE block?
block         := INDENT mapping DEDENT
               | INDENT sequence DEDENT
               | sequence
inline        := flow_sequence | flow_mapping | double_quoted | single_quoted | PLAIN
flow_sequence := "[" ( flow_node ( "," flow_node )* ","? )? "]"
flow_mapping  := "{" ( flow_pair ( "," flow_pair )* ","? )? "}"
flow_pair     := flow_key ( ":" flow_node? )?
flow_node     := TAG? inline
key           := TAG? ( double_quoted | single_quoted | PLAIN_KEY )
flow_key      := TAG? ( double_quoted | single_quoted | FLOW_KEY )
```

- `INDENT` is a line indented more than the key above it; `DEDENT` is the return to
  an earlier indentation. Every key of one mapping starts at the same column.
- A `block` may be a sequence written at the same column as its key (its third
  form).
- A key with nothing after its colon and no block under it is null, and so is a `-`
  with nothing after it, and a pair of a flow mapping with no `:` or nothing after
  it.
- A list entry whose line goes on with a plain `key:` holds a mapping. Its first key is on
  the `-` line, and its other keys start at that key's column on the lines below.
- `PLAIN_KEY` runs to the first `:` followed by a space or by the end of the line.
  `FLOW_KEY` runs to the first `:` followed by a space or by one of `, [ ] { }`, or
  to the first of `, [ ] { }`.
- `PLAIN` runs to the end of the line, or to a comment (a space then `#`); inside a
  flow collection it stops at `,`, `[`, `]`, `{` or `}`. It cannot hold `: ` or end
  with `:`; such a value fails the file with ``a plain value cannot hold `: `; quote
  it``.
- A flow collection may run over several lines: line breaks and comments may sit
  between its entries, and a `,` may follow the last one.
- `double_quoted` reads the escapes `\0 \a \b \t \n \v \f \r \e \" \/ \\`, a
  backslash before a space, `\xXX`, `\uXXXX` and `\UXXXXXXXX`. Inside
  `single_quoted`, `''` is one quote.
- Blank lines and lines holding only a comment are skipped.
- A key written more than once becomes one key holding a list of every value
  written for it, by the same rule as in TOML: `tags: [a, b]` then `tags: c` is
  `tags: [a, b, c]`.
- An anchor (`&name`) or an alias (`*name`), on a value or on a key, fails the file
  with `anchors and aliases are not read`.

`BLOCK_SCALAR` is read whole, by YAML 1.2.2 section 8.1:

- Its header is `|` (literal) or `>` (folded), then in either order an optional
  chomping indicator, `-` (strip) or `+` (keep), and an optional indentation digit
  from 1 to 9, then the end of the line.
- Its lines are those indented more than the key or the `-` it belongs to. The
  column of its text is that key's or that `-`'s column plus the digit; with no
  digit, it is the column of the first line of text, and a blank line before that
  line may not be indented more. The scalar ends before the first line of text
  indented less than that column.
- A literal scalar keeps every line break. A folded scalar joins two lines of text
  with a space, turns each empty line into a line break, and keeps the line breaks
  around a line indented more than the text.
- Clip, the default, keeps the line break after the last line of text and drops the
  empty lines after it. Strip drops both. Keep keeps both.

`TAG` is `!!` followed by `str`, `int`, `float`, `bool`, `null`, `map`, `seq` or
`timestamp`, or the same type written in full, such as `!<tag:yaml.org,2002:str>`.
Any other tag, such as `!point`, fails the file with
``the tag `!point` is not one archie reads``.

- A tagged scalar's text is read again by the rules for an unquoted value of the
  tag's type in the table below: `!!str 01234` is the String `"01234"`, and
  `!!timestamp` reads a Date or a DateTime. `!!float` also reads the text of a
  decimal integer, so `!!float 1` is the Float `1`. `!!map` and `!!seq` only check
  that the value is a mapping or a list.
- A value that does not fit its tag fails the file, as
  ``"`abc` is not a value of the tag `!!int`"``, or
  ``"a list is not a value of the tag `!!map`"``.
- A tag on a key checks the key's text the same way.

A `PLAIN` value's type comes from how it is written:

| Written as | Type |
|---|---|
| Nothing, `~`, `null`, `Null`, `NULL` | Null |
| `true`, `True`, `TRUE`, `false`, `False`, `FALSE` | Boolean |
| An optional sign and digits; or `0x`, `0o` and digits in that base | Integer, 64 bits |
| Digits with a fraction, an exponent or both; `.inf` and `.nan` | Float |
| `2026-10-10`, when that day exists in the calendar | Date |
| A date-time, written as in TOML, whose date exists in the calendar | DateTime |
| Anything else | String |

## What each node's span covers

| Node | From | To |
|---|---|---|
| `Document` | The first byte of the file | The last |
| `Frontmatter` | The opening delimiter | The end of the closing delimiter |
| `Mapping` written as lines | The first key | The end of the last value written |
| `Mapping` with no pairs | Empty, on the line after the opening delimiter | |
| `Mapping` written as `{ ... }` | `{` | `}` |
| `Pair` | Its key | The end of its value, where the key is first written; `repeats` holds the same range for each later occurrence |
| `Sequence` written as `[ ... ]` | `[` | `]` |
| `Sequence` of the values of a key written more than once | The first value's start | The first value's end |
| `Sequence` written as `-` lines | The first `-` | The end of the last value |
| `Scalar` | Its first character, quotes included | Its last |
| `Scalar` that is null because nothing is written | Empty, right after the `:` or the `-` | |
| `Scalar` written as a block scalar | `\|` or `>` | The end of its last line of text, or of its header when it has none |
| `Scalar`, `Mapping` or `Sequence` after a tag | The value, without its tag | |
| `Body` | The line after the frontmatter, or the first byte of a file with none | The last byte of the file |
| `Section` | Its heading | The end of its last block |
| `Heading` | The first `#` | The end of the line's text |
| `Paragraph` | Its first character | The end of its last line's text |
| `List` | The first bullet | The end of the last item |
| `Item` | Its bullet | The end of its last block, or of the bullet when the item is empty |
| `CodeBlock` | The opening fence | The end of the closing fence, or of its last line when no fence closes it |
| `Table` | Its first line's text | Its last line's text |
| `TableRow` | The row line's text | Its end |
| `TableCell` | The cell's text without the spaces around it; empty when the cell is | Its end |
| `Text`, `Code`, `Link`, `Image` | The first character, delimiters included | The last |

Apart from `Document` and `Body`, a span never includes the line ending after the
node. Inside a paragraph, a `Text`
that continues onto the next line covers the line ending and the indentation
between the two lines.

## Where it differs

### From CommonMark

1. A line that continues a list item is indented to the item's text. A line further
   left ends the list; CommonMark would continue the paragraph.
2. An item's text column is always the bullet's column plus two, however many
   spaces follow the bullet.
3. Items that follow each other form one list, whichever bullet each uses and
   whether or not blank lines separate them.
4. Indentation outside a list means nothing: text indented four spaces is a
   paragraph, not code. A tab is a character of text, not indentation.
5. A heading is written with `#`, outside any list item. Closing `#`s are part of
   its text.
6. A code block is written with backtick fences only.
7. An inline node does not cross a line break. A link's text holds no brackets
   other than an image's, and a link with a title is text.
8. A table is read only when it is well formed. GitHub fills a short row with empty
   cells and drops the extra cells of a long one; archie reads the whole table as
   text instead. A pipe table also ends at the first line with no pipe.
9. These have no node and stay as text: block quotes, ordered lists, emphasis and
   strong, thematic breaks, underlined headings, reference links, autolinks and
   HTML.

### From Pandoc's tables

1. A grid table whose rows run over several lines is text: a cell written over
   several lines is not one byte range.
2. Simple tables without a header line, and multiline tables, are text.

### From TOML

1. Table headers (`[name]` and `[[name]]`), dotted keys and multi-line strings are
   not read. Each is refused with an error that names it.
2. Numbers are read more loosely: underscores are dropped wherever they are, and
   leading zeros are accepted.
3. A key written more than once becomes one key holding a list of its values. TOML
   does not allow it.

### From YAML

1. Anchors and aliases are refused, and so is every tag other than the eight above.
   Each is refused with an error that names it.
2. A plain value continued on a more indented line, and a quoted string continued
   on a second line, are refused.
3. A pair written inside a flow list, such as `[a: b]`, is refused instead of read
   as a mapping.
4. A key written more than once becomes one key holding a list of its values. YAML
   does not allow it.
5. An unquoted date or date-time whose day exists in the calendar is a Date or a
   DateTime. The YAML 1.2.2 core schema has no date type and reads it as a string.
6. A number written with nothing before the dot, such as `.5`, is a string. YAML
   reads it as a float.

## Extensions

A consumer of archie declares its own subtypes of fenced code blocks by implementing
the `Extension` trait, and parses with `parse_with::<E>`:

```rust
pub trait Extension {
    type CodeBlock: Debug + Clone + PartialEq;
    fn code_block(block: &FencedCode<'_>) -> Option<Result<Self::CodeBlock, ParseError>>;
}
```

- For each `CODE_BLOCK`, the parser hands the extension a `FencedCode`: the info
  string, the source, the range of the lines between the fences, and those lines as
  the block holds them.
- `None` leaves a plain code block. `Some(Ok(..))` becomes the block's `kind`.
  `Some(Err(..))` fails the whole parse with that error.
- The tree types that hold blocks take the extension as a type parameter, which
  defaults to `Plain`. `Plain` claims nothing, and `parse` is `parse_with::<Plain>`.
- An extension reads structured text inside a block with `read_yaml` and
  `read_toml`, which read the YAML and TOML above from any range of a source and
  keep its offsets.

## Choices made without a requirement

Each of these was needed to write the parser and none was asked for.

1. A pair written inside a flow list, such as `[a: b]`, is refused with the error
   for a plain value holding `: `.
2. A frontmatter construct the reader does not know fails the whole file. It is not
   kept as opaque text.
3. A key written more than once keeps the range of its first occurrence in `span` and
   that of each later one in `repeats`. The list of its values keeps the first
   value's range, so the values from a later occurrence lie outside it.
4. TOML table headers are refused instead of read: one mapping written in several
   separate places has no single byte range to put in `span`.
5. The rule for an item's text column, and one list across different bullets.
6. A line break inside a paragraph is a `\n` inside a `Text`, not a node of its
   own.
7. A malformed table is text as a whole, even where GitHub would read it.
8. What each span covers, as listed above. A section's span ends at its last block,
   not at the next heading.
9. The additions to the tree: `CodeBlock`, `Code`, `Image`, `Table` with its rows
   and cells, one `Link` node with a `form`, and `Mapping` as an ordered map with the
   key held outside `Pair`.
10. A grid table whose rows run over several lines is text, because a cell written
    over several lines has no single byte range to put in `span`.
11. A block an extension claims and fails to read fails the whole parse.
12. The tree types that take the extension as a parameter compare with `PartialEq`
    only, so an extension's own types need not support `Eq`.
