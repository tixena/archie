# archie write

`archie write` stores a whole document at a path, read from standard input or from
a named file in one of three forms: markdown, JavaScript Object Notation (JSON),
or rkyv's binary layout. A second form of the command, which writes into the part
of one document that a path expression addresses, is not yet specified; it is
described under [Writing into a part of a document](#writing-into-a-part-of-a-document).

## Writing a whole document

### Synopsis

```console
$ archie write <PATH> [-f, --format <FORMAT>] [--frontmatter <SYNTAX>] [-i, --input <FILE>] [-s, --store <STORE>]
```

The path is required.

### Arguments and options

| Part | What it takes |
|---|---|
| `<PATH>` | Where the document is stored. With no `--store`, a file of the machine, relative to the current folder or absolute. `.` and `..` are resolved from the text of the path, without following links, so `../notes/a.md` names the file a shell would name. With a `--store`, a path of that store, such as `notes/task.md`, used as written. |
| `-f`, `--format <FORMAT>` | The form the document is read in: `markdown`, the default, `json`, or `rkyv`. |
| `--frontmatter <SYNTAX>` | The syntax the stored frontmatter is written in: `yaml`, the default, or `toml`. |
| `-i`, `--input <FILE>` | A file of the machine to read the document from, in place of standard input. It is never a path in a store. |
| `-s`, `--store <STORE>` | The store the document goes to: `minio:<bucket>`, `minio:<bucket>/<prefix>`, or `mongodb:<database>/<collection>`. The file system when it is left out. |

`archie --help` and `archie write --help` print help; `archie --version` prints
the version.

### The forms

| Form | What `archie write` reads |
|---|---|
| `markdown` | Markdown text, with YAML or TOML frontmatter. |
| `json` | The tree as one JSON value, as `archie get --format json` writes it. |
| `rkyv` | The tree in rkyv's binary layout, after a header of eight bytes: `archie`, a zero byte, and the version of the model's types, 1. It is read in place, as `archie get --format rkyv` writes it. |

Whatever the form, the file stored at the path is archie's form of the document:
the markdown archie writes for the tree it read, with the frontmatter in the syntax
`--frontmatter` names.

### Behavior

`archie write` reads all of standard input, or all of the `--input` file, and then
stores the document those bytes hold. A file already at the path is replaced, as a
copy replaces its target. A folder on the path that does not exist is created.

The file is written whole: a temporary file beside it, then a rename over it, so a
reader sees the old file or the new one. Bytes that do not hold a document in the
form store nothing.

`archie write` writes nothing to standard output. An error is one line on standard
error: `archie: `, the path as the command line gave it, and what failed.

```console
$ echo nope | archie write b.md --format json
archie: b.md: expected ident at line 1 column 2
```

### Stores

With no `--store`, the store is the file system.

With `--store mongodb:<database>/<collection>`, the store is the collection
`<collection>` of the database `<database>`, on the MongoDB server that the
environment variable `ARCHIE_MONGODB_URI` names. The command line holds names
only, never an address or a credential.

| Variable | What it holds |
|---|---|
| `ARCHIE_MONGODB_URI` | The connection string of the MongoDB server, `mongodb://` or `mongodb+srv://`, with the server and its credentials. |

A missing `ARCHIE_MONGODB_URI` exits with code 3, and a server that cannot be
reached exits with code 4.

```console
$ export ARCHIE_MONGODB_URI=mongodb://127.0.0.1:27017
$ archie get notes/task.md | archie write notes/task.md --store mongodb:notes/files
$ unset ARCHIE_MONGODB_URI; archie write notes/task.md -s mongodb:notes/files < a.md; echo "exit $?"
archie: ARCHIE_MONGODB_URI is not set: it holds the MongoDB connection string
exit 3
```

This build of archie does not have the MinIO store: a `--store minio:` value exits
with code 3. An archie built without its default features does not have the
MongoDB store either, and refuses a `--store mongodb:` value the same way.

```console
$ archie write b.md --store minio:notes/drafts < a.md
archie: minio:notes/drafts: this archie is built without the MinIO store
```

A `--store` value of any other shape is a usage error, code 2.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success. |
| 1 | The document could not be read or written in the form: not UTF-8, not JSON, not this version of the rkyv form, or a value the frontmatter syntax cannot hold. |
| 2 | The command line is wrong: an unknown option, a missing path, a value an option does not take, or a path that cannot name a file, such as `/`. |
| 3 | The store is not configured, or its configuration is wrong: `ARCHIE_MONGODB_URI` is not set for `--store mongodb:`, or the store is not built into this archie, as the MinIO store is not. |
| 4 | The store failed: a file system error, such as a folder that is not writable, or a MongoDB server that cannot be reached. A failure to read standard input is code 4 too. |
| 6 | No file at the path of `--input`. |

Code 5 belongs to the language-model provider error of the validation run; see
[Exit codes](../exit-codes.md).

### Examples

The copy: [`archie get`](get.md) piped into `archie write` stores archie's form of
the source at the target, in each form when `--format` is the same on both sides,
and a second copy changes nothing.

```console
$ archie get notes/task.md | archie write copy.md
$ archie get copy.md | archie write again.md && cmp copy.md again.md && echo same
same
$ archie get notes/task.md -f rkyv | archie write copy.md -f rkyv
```

The same copy through a JSON file:

```console
$ archie get notes/task.md --format json --output task.json
$ archie write again.md -f json -i task.json
```

## Writing into a part of a document

*This form is not yet specified, and archie does not have it yet: `--path`,
`--append`, and `--from-body` are usage errors, code 2.*

### Synopsis

```console
$ archie write folder/file.md --path='# ##["Specs"][2]' --append --from-body='path-to-file.txt'
```

### Arguments and options

| Part | What it takes |
|---|---|
| `folder/file.md` | The document to write into. One file. |
| `--path=<expr>` | A path expression addressing the part of the document to write into. |
| `--append` | Append to the addressed part rather than replacing it. |
| `--from-body=<file>` | Take the content to write from the named file. |

### The path expression

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

### Behavior

Every write into a part of a document is a [change object](../change-objects.md):
a base document plus a chain of changes applied in order reproduces the latest
version byte for byte, and unapplying the chain in reverse returns the original.
The guarantee holds because the content is linted markdown with a known shape.

archie preserves the formatting of frontmatter it did not write. remargin's write
path emits archie's canonical form instead.

### Not yet specified

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

- [archie get](get.md) — the document at a path, in one of the three forms.
- [Query grammar](../query-grammar.md) — the path expression in full.
- [archie query](query.md) — the language that also drives modification.
- [Change objects](../change-objects.md) — the transactional model every write
  into a part of a document produces.
- [Shape language](../shape-language.md) — shapes declared for a document or a
  sub-part of one.
- [Command-line interface](index.md) — global options and the command table.
