# archie get

`archie get` reads the document at a path and writes it to standard output, or to
a named file, in one of three forms: markdown, JavaScript Object Notation (JSON),
or rkyv's binary layout.

## Synopsis

```console
$ archie get <PATH> [-f, --format <FORMAT>] [--frontmatter <SYNTAX>] [-o, --output <FILE>] [-s, --store <STORE>]
```

The path is required.

## Arguments and options

| Part | What it takes |
|---|---|
| `<PATH>` | The document to read. With no `--store`, a file of the machine, relative to the current folder or absolute. `.` and `..` are resolved from the text of the path, without following links, so `../notes/a.md` names the file a shell would name. With a `--store`, a path of that store, such as `notes/task.md`, used as written. |
| `-f`, `--format <FORMAT>` | The form the document is written in: `markdown`, the default, `json`, or `rkyv`. |
| `--frontmatter <SYNTAX>` | The syntax the frontmatter is written in, in the markdown form: `yaml`, the default, or `toml`. |
| `-o`, `--output <FILE>` | A file of the machine to write the document to, in place of standard output. It is never a path in a store. |
| `-s`, `--store <STORE>` | The store the document is in: `minio:<bucket>`, `minio:<bucket>/<prefix>`, or `mongodb:<database>/<collection>`. The file system when it is left out. |

`archie --help` and `archie get --help` print help; `archie --version` prints the
version.

## The forms

| Form | What `archie get` writes |
|---|---|
| `markdown` | archie's form of the document: the text archie writes for the tree it read, with the frontmatter in the syntax `--frontmatter` names. |
| `json` | The tree as one JSON value, followed by a newline. |
| `rkyv` | The tree in rkyv's binary layout, after a header of eight bytes: `archie`, a zero byte, and the version of the model's types, 1. |

[`archie write`](write.md) reads the same three forms, so a document that leaves
archie in a form comes back in it.

## Output

Standard output carries only the document's bytes. They go out unchanged, rkyv
included, whether standard output is a terminal or not.

Nothing is written to standard output or to the `--output` file until the bytes
are ready, so a failed `archie get` leaves no partial output. The `--output` file
is written whole: a temporary file beside it, then a rename over it. A folder on
its path that does not exist is created, and a file already there is replaced.

An error is one line on standard error: `archie: `, the path as the command line
gave it, and what failed.

```console
$ archie get missing.md
archie: missing.md: no file at this path
```

## Stores

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
$ archie get notes/task.md --store mongodb:notes/files
$ unset ARCHIE_MONGODB_URI; archie get notes/task.md -s mongodb:notes/files; echo "exit $?"
archie: ARCHIE_MONGODB_URI is not set: it holds the MongoDB connection string
exit 3
```

This build of archie does not have the MinIO store: a `--store minio:` value exits
with code 3. An archie built without its default features does not have the
MongoDB store either, and refuses a `--store mongodb:` value the same way.

```console
$ archie get notes/task.md --store minio:notes
archie: minio:notes: this archie is built without the MinIO store
```

A `--store` value of any other shape is a usage error, code 2.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success. |
| 1 | The document could not be read or written in the form: not UTF-8, not JSON, not this version of the rkyv form, or a value the frontmatter syntax cannot hold. |
| 2 | The command line is wrong: an unknown option, a missing path, a value an option does not take, or a path that cannot name a file, such as `/`. |
| 3 | The store is not configured, or its configuration is wrong: `ARCHIE_MONGODB_URI` is not set for `--store mongodb:`, or the store is not built into this archie, as the MinIO store is not. |
| 4 | The store failed: a file system error, such as a folder that is not writable or a path that is a folder, or a MongoDB server that cannot be reached. A failure to write standard output is code 4 too. |
| 6 | No file at the path. |

Code 5 belongs to the language-model provider error of the validation run; see
[Exit codes](../exit-codes.md).

## Examples

The copy: `archie get` piped into [`archie write`](write.md) stores archie's form
of the source at the target, and a second copy changes nothing.

```console
$ archie get notes/task.md | archie write copy.md
$ archie get copy.md | archie write again.md && cmp copy.md again.md && echo same
same
```

The same copy through a JSON file:

```console
$ archie get notes/task.md --format json --output task.json
$ archie write again.md -f json -i task.json
```

A missing file:

```console
$ archie get missing.md; echo "exit $?"
archie: missing.md: no file at this path
exit 6
```

## Not yet specified

- What `archie get` returns for a folder, an image, or a document in Portable
  Document Format. Today a folder fails with code 4, and a file that is not UTF-8
  with code 1.

See [Design status](../../design-status.md) for the open questions around this
command.

## Related

- [archie write](write.md) — storing a document given in one of the three forms.
- [archie ls](ls.md) — listing what can be retrieved.
- [archie query](query.md) — selecting documents by an expression.
- [Documents and schemas](../../concepts/documents-and-schemas.md) — the document
  as an abstraction independent of a file.
- [Command-line interface](index.md) — global options and the command table.
