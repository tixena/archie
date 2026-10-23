# Command-line interface

archie is one program, invoked as `archie <COMMAND>`.

```console
$ archie <COMMAND>
```

## Global options

These four options belong to the program rather than to any one command.

| Option | Effect |
|---|---|
| `-c`, `--config <FILE>` | Path to the configuration file. Default: `archie.toml`. Accepted before and after the command name. |
| `-v`, `--verbose` | Increase verbosity. |
| `-q`, `--quiet` | Suppress non-error output. |
| `--version` | Print the version. |

## Configuration and environment

archie reads a configuration file named `archie.toml` at the repository root; its
settings are documented on the [Configuration](../config.md) page. Configuration
sources take precedence in this order, highest first:

1. Options given on the command line.
2. Environment variables.
3. The `archie.toml` file.
4. Built-in defaults.

`--config` wins over `ARCHIE_CONFIG`, which wins over the `archie.toml` in the
current folder. Every command reads the file when it starts. A file that `--config`
or `ARCHIE_CONFIG` names and that does not exist, or a file that does not read,
exits with code 3 and one line; a missing `archie.toml` that nothing named is not
an error.

Eleven environment variables are read:

| Variable | Effect |
|---|---|
| `ARCHIE_CONFIG` | Path to the configuration file. |
| `ARCHIE_ROOT` | Corpus root path. |
| `ANTHROPIC_API_KEY` | Credential for the language-model provider used by `archie inbox`. |
| `OPENAI_API_KEY` | Credential for the language-model provider used by `archie inbox`. |
| `NO_COLOR` | Turns colored output off. |
| `ARCHIE_MONGODB_URI` | The connection string of the MongoDB server, `mongodb://` or `mongodb+srv://`, with the server and its credentials, used by `archie get` and `archie write` with `--store mongodb:`, and by `archie asset finish-deletions`. |
| `ARCHIE_MONGODB_BUCKET` | The MinIO bucket of a MongoDB store's files, `<bucket>` or `<bucket>/<prefix>`, used by `archie get` and `archie write` with `--store mongodb:`, and by `archie asset finish-deletions`. |
| `ARCHIE_MINIO_ENDPOINT` | The address of the MinIO server, such as `http://127.0.0.1:9000`, used by `archie get` and `archie write` with `--store minio:` or `--store mongodb:`, and by `archie asset finish-deletions`. |
| `ARCHIE_MINIO_REGION` | The region the MinIO server expects, such as `us-east-1`, used by `archie get` and `archie write` with `--store minio:` or `--store mongodb:`, and by `archie asset finish-deletions`. |
| `AWS_ACCESS_KEY_ID` | The access key every MinIO request is signed with, used by `archie get` and `archie write` with `--store minio:` or `--store mongodb:`, and by `archie asset finish-deletions`. |
| `AWS_SECRET_ACCESS_KEY` | The secret key every MinIO request is signed with, used by `archie get` and `archie write` with `--store minio:` or `--store mongodb:`, and by `archie asset finish-deletions`. |

## Output formats

Three output formats exist: plain text, which is the default, JavaScript Object
Notation (JSON), and markdown. [`archie check`](check.md) selects among them with
`--format`, and the configuration file carries a default under its `[output]`
heading; each is shown in full on the [Output formats](../output-formats.md)
page. Three commands instead emit a single fixed report with no format choice —
the inheritance chain from [`archie trace`](trace.md), the interactive proposal
from [`archie inbox`](inbox.md), and the export document from
[`archie export`](export.md) — each shown on its own page. [`archie get`](get.md)
and [`archie write`](write.md) take `--format` with forms of their own instead:
`markdown`, the default, `json`, `binary`, or `zip`.

Each of those forms carries every file the document embeds, such as the picture
of `![Logo](media/logo.png)`. Markdown to or from a stream, and JSON, carry each
file inside its embed as a data URI; markdown to or from a file carries each file
beside it, at its path from the file's folder, unless `archie get --embed` asks for
the one file with every data URI in it; binary and zip carry the files inside
their bytes. Both commands refuse, with code 1, a document whose embedded file is
missing or whose embed leaves the document's folder, and `--allow-missing` keeps
such an embed as written:

```console
$ archie get notes/report.md -o out/report.md
$ archie get notes/report.md -o one.md --embed
$ archie get notes/report.md -f zip -o report.zip
$ archie write copy/report.md -i out/report.md
$ printf '![Logo](media/none.png)\n' | archie write notes/a.md --allow-missing
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success, no issues. |
| 1 | Validation errors found. |
| 2 | Warnings only, when `--fail-on warning` was given. |
| 3 | Configuration error. |
| 4 | File system error. |
| 5 | Language-model provider error. |

The table is written for the validation run; see
[Exit codes](../exit-codes.md).

`archie get` and `archie write` exit with their own codes, 0 to 4 and 6, given on
[their pages](get.md#exit-codes), `archie asset` with its own, 0 and 2 to 4,
given on [its page](asset.md#exit-codes), and `archie fix` with its own, 0 and 2 to 4,
given on [its page](fix.md#exit-codes).

## The commands

| Command | What it does |
|---|---|
| [`archie ls`](ls.md) | Lists the contents of a corpus. |
| [`archie get`](get.md) | Writes one markdown document, with every file it embeds, to standard output or a file, in markdown, JSON, binary or zip. |
| [`archie query`](query.md) | Selects documents by an expression over their frontmatter, in a language that also drives modification. |
| [`archie search`](search.md) | Matches a pattern against document bodies and frontmatter. |
| [`archie write`](write.md) | Stores one whole markdown document, with every file it embeds, read from standard input or a file. |
| [`archie asset`](asset.md) | Commands on the files that documents embed: `archie asset finish-deletions` finishes each deletion of such a file that stopped halfway in a MongoDB store. |
| [`archie rm`](rm.md) | Removes a file or a folder. |
| [`archie link`](link.md) | Wraps mentions of a resource in links to that resource. |
| [`archie check`](check.md) | Validates a corpus, or one path within it. |
| [`archie init`](init.md) | Sets up a corpus: the configuration file and any missing folder index files. |
| [`archie fix`](fix.md) | Repairs the findings that need no human decision, and removes the locks saves left behind. |
| [`archie inbox`](inbox.md) | Classifies unfiled captures with a language model and proposes where each one goes. |
| [`archie trace`](trace.md) | Prints the chain of inherited properties for one file. |
| [`archie export`](export.md) | Serializes a corpus for another program to read. |
| [`archie import`](import.md) | Pulls items in from an external source. |
| [Structural operations](structural-operations.md) | Move, rename, archive, and merge as transactions that keep links valid. |

`archie help` prints help and has no page of its own.

archie also exposes a Model Context Protocol (MCP) server, documented in the
[Model Context Protocol server](../mcp/index.md) reference.

## Not yet specified

- What `--verbose` adds, whether repeating it increases verbosity further, and
  what happens when `--verbose` and `--quiet` are given together.
- Whether `--verbose` and `--quiet` are accepted before the command name, after
  it, or in both positions.
- Which language-model provider credential, `ANTHROPIC_API_KEY` or
  `OPENAI_API_KEY`, is consulted when both are set.
- Whether a path argument defaults to the configured corpus root for commands
  other than `check`, `fix`, `inbox`, and `export`; whether it may be a glob; and
  whether several paths may be given. None of this is open for `archie get` and
  `archie write`: each takes exactly one path, and requires it.
- Standard input and standard output as an interface for commands other than
  `archie get`, `archie write`, `archie asset finish-deletions` and `archie fix`.
  Their pages specify them: `archie get` writes to standard output, `archie write`
  reads from standard input, `archie asset finish-deletions` writes one object key
  per line to standard output, and `archie fix` writes the path of each lock it
  removed, one per line.
- Which exit codes apply to commands other than the validation run,
  `archie get`, `archie write`, `archie asset finish-deletions` and `archie fix`.
  The codes of those four are on their pages.

See [Design status](../../design-status.md) for the open questions here.

## Related

- [Configuration](../config.md) — the `archie.toml` file and its settings.
- [Output formats](../output-formats.md) — the three formats in full.
- [Exit codes](../exit-codes.md) — the code table.
- [Query grammar](../query-grammar.md) — the expression language shared by
  `archie query`, `archie search`, and `archie write`.
- [archie and remargin](../../concepts/archie-and-remargin.md) — where the
  boundary between the two runs.
