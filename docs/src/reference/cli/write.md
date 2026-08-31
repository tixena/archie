# archie write

`archie write` writes into the part of one document that a path expression
addresses. It is the one command that operates on a sub-part of a document rather
than on whole documents.

## Synopsis

```console
$ archie write folder/file.md --path='# ##["Specs"][2]' --append --from-body='path-to-file.txt'
```

## Arguments and options

| Part | What it takes |
|---|---|
| `folder/file.md` | The document to write into. One file. |
| `--path=<expr>` | A path expression addressing the part of the document to write into. |
| `--append` | Append to the addressed part rather than replacing it. |
| `--from-body=<file>` | Take the content to write from the named file. |

## The path expression

A path expression combines a heading level written as a run of `#` characters, a
heading title written as `["Specs"]`, and an index written as `[2]`. The example
above selects the level-one heading, then the level-two heading titled "Specs"
beneath it, then the second child block under that heading.

Addressing a part of a document is what makes two other things possible. It is
what lets the query language reach inside a document instead of stopping at the
document boundary. And it is the unit remargin's permission system grants on:
view or edit access is given on files and on parts of markdown documents, and a
reader sees only the accessible portion. A shape can be declared for a whole
document or for whatever a path addresses, so the same notation bounds what a
schema governs.

## Behavior

Every write is a [change object](../change-objects.md): a base document plus a
chain of changes applied in order reproduces the latest version byte for byte,
and unapplying the chain in reverse returns the original. The guarantee holds
because the content is linted markdown with a known shape.

archie preserves the formatting of frontmatter it did not write. remargin's write
path emits archie's canonical form instead.

## Not yet specified

- What else a path may address: list items, paragraphs, tables, code fences,
  remargin comment blocks, and frontmatter keys.
- What `[2]` counts — the second child element under that heading, the second
  heading carrying that title, or something else.
- Whether the path expression and the query expression are one language or two.
- The mutation verbs beyond appending. `append`, `prepend`, `set`, `remove`, and
  `replace` are the candidates on record.
- Whether a set of documents may be given rather than one file, and whether
  content may be given inline or read from standard input.
- Whether one query-driven mass modification is a single change object or one per
  document touched.
- Output, and the exit codes for a path expression that addresses nothing and for
  a document that does not match the shape a write assumes.

Field-editing commands — setting a field, removing a field, adding to an array,
removing from an array, and a bulk edit driven by a `--where` option — are not
archie's shape; the query language drives modification instead.

See [Design status](../../design-status.md) for the open questions around this
command.

## Related

- [Query grammar](../query-grammar.md) — the path expression in full.
- [archie query](query.md) — the language that also drives modification.
- [Change objects](../change-objects.md) — the transactional model every write
  produces.
- [Shape language](../shape-language.md) — shapes declared for a document or a
  sub-part of one.
- [Command-line interface](index.md) — global options and the command table.
