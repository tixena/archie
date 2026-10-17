# archie-query

`archie-query` is a library crate that filters documents by their frontmatter. It
implements the comparison subset of the [query grammar](query-grammar.md): the
`fm.` clauses and the `and`, `or`, and `not` that combine them. Body clauses,
chaining, projection, and mutation belong to the wider language and are not part
of this crate.

The crate reads frontmatter through archie's own parser, so a query runs the same
over a document whose frontmatter is YAML and one whose frontmatter is TOML. A
document with no frontmatter has no fields.

## A query

A query is one or more comparisons joined by `and`, `or`, and `not`.

```
fm.type == "issue" and fm.status == "open"
```

A comparison has three parts: a field path, an operator, and a value.

```
fm.KEY OP VALUE
```

### Field paths

A path starts with `fm` and names a key after each dot. `fm.phone.work` reads the
`work` key of the `phone` mapping. A key is a run of letters, digits, `_`, and
`-`, so `fm.2fa` and `fm.has` are valid. Quote a key that holds other
characters: `fm."due date"`.

A path that does not lead to a field, because a key is absent or a step runs
through something that is not a mapping, names no field.

### Values

| Value | Written as | Notes |
|---|---|---|
| String | `"text"` | Escapes: `\"`, `\\`, `\n`, `\t`. Any other escape is an error. |
| Integer | `42`, `-3`, `0x10`, `0o7`, `0b101` | Hexadecimal, octal and binary forms are TOML's. |
| Float | `1.5`, `-0.25`, `1e3`, `-inf`, `-nan` | `-inf` compares as negative infinity. `-nan` never matches. |
| Boolean | `true`, `false` | |
| Date | `2031-05-01` | The day must exist in that month. |
| Datetime | `2031-05-01T09:30:00Z` | RFC 3339: `Z` or `+HH:MM` or `-HH:MM`, optional fraction. With no offset, the time counts as UTC. |

Numbers follow TOML syntax, read by archie-core. A word starts with a digit or
`-`, so `_` separators and a leading `+` are not available. `5.` and `.5` are
not numbers.

A date or datetime is written bare, without quotes. `fm.due < "2031-05-01"`
compares as a string and never matches a date field.

### Operators

| Operator | True when |
|---|---|
| `==` | The field equals the value. |
| `!=` | The field does not equal the value, or the field is missing. |
| `<`, `<=`, `>`, `>=` | The field orders before, before or equal, after, or after or equal the value. |
| `has` | The field is a list with an item equal to the value, or a string that contains the value as text. |

`has` takes the same value forms as the other operators. Against a list it
tests membership; against a string, the value must be a string and the test is a
substring match. Against any other field it is false.

### Combining comparisons

`not` binds tightest, then `and`, then `or`. Parentheses group.

```
fm.status == "open" and (fm.kind == "bug" or fm.kind == "crash")
not fm.status == "closed"
```

A query nests at most 64 levels of `(` and `not`, and holds at most 256
comparisons. A query past either limit is an error.

## How values compare

A missing field makes every operator false except `!=`, which is true.

A value only compares with a value of its own type:

- Strings compare by their bytes.
- Integers and floats compare with each other exactly, without rounding. A float
  `NaN` is equal to and ordered against nothing, so `!=` is true.
- Booleans compare with booleans.
- Types that do not compare, such as a string field and an integer value, are
  never equal and never ordered. `==`, `<`, `<=`, `>`, and `>=` are false, and
  `!=` is true.

A date or datetime value is the exception. It compares against a date field, a
datetime field, or a string field written as a date or datetime, such as
`"2031-05-01"` in quotes. Every other field type does not compare with it,
including a time of day. The comparison works on instants:

- A date counts as midnight at UTC.
- A datetime with an offset counts as that instant, so `2031-05-01T09:00:00+02:00`
  equals `2031-05-01T07:00:00Z`.
- A datetime with no offset counts as UTC.

## Examples

Open issues of a bug:

```
fm.type == "issue" and fm.bug == "B-12" and fm.status != "closed"
```

The `!=` also matches issues that have no `status`.

Tasks due before a date. A task with no `due` field does not match:

```
fm.type == "task" and fm.due < 2031-05-01
```

Notes tagged `rule`:

```
fm.tags has "rule"
```

## Using the crate

Parse a query once and test any number of documents against it.

```rust
use archie_query::query::Query;

let query = Query::parse(r#"fm.tags has "rule""#)?;
let hit = query.matches_source(source)?;
```

`Query::matches_source` reads the frontmatter of a document held as text.
`Query::matches` tests a `Resource` that is already read; `Resource::from_source`
returns `None` for a document with no frontmatter. `Resource::kind` holds the
`type` key when it is a string.

### Errors

`Query::parse` returns a `QueryError` that holds a message and the byte span
where the query stops making sense. `Query::matches_source` returns a
`QuerySourceError` when the frontmatter cannot be read, with the span in the
document.

## Paging

`page::page` cuts a sorted list of keys, such as document paths, into pages. The
caller sorts the keys ascending and passes the list, the cursor from the previous
page, and a limit.

```rust
let (keys, next) = archie_query::page::page(&paths, cursor.as_deref(), 50);
```

It returns the keys of the page and the cursor for the next one.

- A page holds the keys that sort after the cursor's key, up to the limit.
- The returned cursor is `Some` only when more keys follow, so a missing cursor
  means the end.
- A limit of 0 counts as 1, so a page is never empty while keys remain and a
  missing cursor never ends a listing early.
- A cursor is the last returned key in base64url with no padding. The key it
  names need not exist, so a key removed between two pages makes the next page
  neither repeat nor skip another.
- A cursor that does not decode counts as no cursor, and the page starts at the
  first key.
- `page::cursor_key(cursor)` returns the key a cursor continues after, or `None`
  when it does not decode, so a caller can check a cursor without knowing its
  encoding.

## Related

- [Query grammar](query-grammar.md): the full expression language this crate
  implements a part of.
- [archie query](cli/query.md): the command that runs a query over documents.
