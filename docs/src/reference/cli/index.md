# Command-line interface

**Status:** Mixed — see markers.

archie is one program invoked as `archie <COMMAND>`. The commands documented in
this section come from two places in the design corpus, and only partly overlap.
The first is the command set written into the root design document. The second is
the list of commands Eduardo Burgos wrote on 2026-08-30 on the planning document,
in answer to a request for the things he would actually run in his first week
with the tool. Where the two disagree, this section records both spellings rather
than choosing between them.

No archie binary exists. Every command line in this section is an intended one,
not one that has been run, and no page shows program output except where the root
design document specifies it.

## Invocation

```console
$ archie <COMMAND>
```

**Root design.** Section 2.1 of the root design document gives the invocation as
`archie <COMMAND>` followed by the global options below, and names eight
commands: `check`, `fix`, `inbox`, `init`, `trace`, `export`, `import`, and
`help`. `archie help` prints help; it is listed with the others and gets no page
of its own in this book.

**Proposed.** Eduardo's 2026-08-30 list adds `ls`, `get`, `lint`, `link`,
`search`, `write`, `rm`, and `query`, and repeats `init`. He stated on the same
document on 2026-08-30 that the list is not written in stone — the verbs and the
syntax can change and are not a hard requirement — and that statement is recorded
as a settled point on the grammar document, where it is numbered G4. So every
command Eduardo named is Proposed, even where something about its behavior is
Decided.

## Global options

**Root design.** These four options are stated in section 2.1 of the root design
document as options of the program rather than of any one command. No document in
the corpus revisits them after 2026-08-25, the date on which Eduardo placed
archie's responsibilities as an engine with the command line on top.

| Option | What the root design says |
|---|---|
| `-c`, `--config <FILE>` | Path to the configuration file. Default: `archie.toml`. |
| `-v`, `--verbose` | Increase verbosity. |
| `-q`, `--quiet` | Suppress non-error output. |
| `--version` | Print the version. |

The corpus does not say what `--verbose` adds to the output, whether repeating it
increases verbosity further, or what happens when `--verbose` and `--quiet` are
given together. It also does not say whether the global options are accepted
before the command name, after it, or in both positions.

> **Suggestion (documentation author):** the global option set above predates the
> direction Eduardo set on 2026-08-25, and none of the commands he named on
> 2026-08-30 was written with it in mind. Eduardo would have to decide whether
> these four options remain the program-wide contract that every command honors —
> including the commands he named — or whether the engine-first direction replaces
> them with something else.

## Configuration and environment

**Root design.** archie reads a configuration file named `archie.toml` at the
repository root (section 5.1). What that file holds is documented on the
[Configuration](../config.md) page.

**Root design.** Section 5.2 gives the order in which configuration sources take
precedence, highest first:

1. Options given on the command line.
2. Environment variables.
3. The `archie.toml` file.
4. Built-in defaults.

**Root design.** Section 5.3 names five environment variables:

| Variable | What the root design says |
|---|---|
| `ARCHIE_CONFIG` | Path to the configuration file. |
| `ARCHIE_ROOT` | The corpus root path. |
| `ANTHROPIC_API_KEY` | Credential for the language-model provider used by `archie inbox`. |
| `OPENAI_API_KEY` | Credential for the language-model provider used by `archie inbox`. |
| `NO_COLOR` | Turns colored output off. |

The corpus does not say what happens when both `ARCHIE_CONFIG` and `--config` are
given, beyond the precedence order above, nor which of the two credential
variables is consulted first when both are set.

## Output formats

**Root design.** Three output formats are specified, in section 6: plain text,
which is the default, JavaScript Object Notation (JSON), and markdown. The
`archie check` command selects among them with `--format`, and the configuration
file carries a default under its `[output]` heading. Each format is shown in full
on the [Output formats](../output-formats.md) page.

The corpus specifies output only for the validation run. No other command in this
section has a specified output shape.

## Exit codes

**Root design.** Section 10.1 gives one table of exit codes for the program:

| Code | Meaning |
|---|---|
| 0 | Success, no issues. |
| 1 | Validation errors found. |
| 2 | Warnings only, when `--fail-on warning` was given. |
| 3 | Configuration error. |
| 4 | File system error. |
| 5 | Language-model provider error. |

The table is written in the language of the validation run, and the corpus does
not restate it for any other command. The [Exit codes](../exit-codes.md) page
carries the full table and what is known about each code.

## The commands

Each row links to the page that documents the command. The status column uses the
book's three markers, plus the phrase "root-design-only, fate open" for a command
that appears in the root design document, is absent from Eduardo's 2026-08-30
list, and falls under the vision blueprint's open question about what becomes of
archie's features for maintaining a personal corpus.

| Command | What the corpus says | Status |
|---|---|---|
| [`archie ls`](ls.md) | Lists the contents of a corpus. No behavior is described anywhere. | Proposed |
| [`archie get`](get.md) | Retrieves a folder, a markdown file, an image, or a document in Portable Document Format. | Proposed |
| [`archie query`](query.md) | Selects documents by an expression over their frontmatter, in a language that also drives modification. | Proposed, with a decided starting point |
| [`archie search`](search.md) | Matches a pattern against documents. | Proposed, with a decided scope |
| [`archie write`](write.md) | Writes into the part of one document that a path expression addresses. | Proposed |
| [`archie rm`](rm.md) | Removes a file or a folder. No behavior is described anywhere. | Proposed |
| [`archie link`](link.md) | Turns mentions of a resource into links to that resource. | Proposed |
| [`archie check`](check.md) | Validates a corpus or one path within it. Eduardo's list has `lint` in this position. | Root-design-only, fate open |
| [`archie init`](init.md) | Sets up a corpus: the configuration file and any missing folder index files. | Proposed |
| [`archie fix`](fix.md) | Repairs what can be repaired without a human decision. | Root-design-only, fate open |
| [`archie inbox`](inbox.md) | Classifies unprocessed captures with a language model and proposes where each one is filed. | Root-design-only, fate open |
| [`archie trace`](trace.md) | Prints the chain of inherited properties for one file. | Root-design-only, fate open |
| [`archie export`](export.md) | Serializes a corpus for another program to read. | Root-design-only, fate open |
| [`archie import`](import.md) | Pulls items in from an external source. | Root-design-only, fate open |
| [Structural operations](structural-operations.md) | Move, rename, archive, and merge as transactions that keep links valid. No command spelling exists. | Proposed |

`archie help` prints help. It is named in the root design document's command list
and has no page in this book.

## Where the corpus is silent

- **Two spellings of the same call.** Eduardo has written `archie query` twice, in
  two shapes: `archie query 'tags contains "rule"' .grit/patterns/` on 2026-08-16,
  where the expression and the path are positional arguments, and
  `archie query --exp='type=issue'` on 2026-08-30, where the expression is a
  named option and no path appears. No document reconciles them, and the grammar
  document lists the spelling of the language itself as unspecified.
- **Argument conventions.** Nothing states whether a path argument defaults to the
  configured corpus root for commands other than `check`, `fix`, `inbox`, and
  `export`, whether a path may be a glob, or whether several paths may be given.
- **Reading from and writing to a pipe.** No document mentions standard input or
  standard output as an interface, except that `archie write` takes its new
  content from a file named by an option.
- **The command line as a whole.** The vision blueprint asks whether the schema
  and frontmatter engine should be designed around remargin's interface from the
  outset with the command line as a thin shell over it. Until that is settled, the
  shape of this interface is not fixed.

archie also exposes a Model Context Protocol (MCP) server. Eduardo said so in
chat on 2026-08-31; no design document records it. That server is documented in
the [Model Context Protocol server](../mcp/index.md) reference, and the gap is
listed on the [Design status](../../design-status.md) page.

## Related pages

- [Configuration](../config.md) — the `archie.toml` file and its settings.
- [Output formats](../output-formats.md) — the three formats in full.
- [Exit codes](../exit-codes.md) — the code table and what is known about it.
- [Query grammar](../query-grammar.md) — the expression language shared by
  `archie query`, `archie search`, and `archie write`.
- [archie and remargin](../../concepts/archie-and-remargin.md) — where the
  boundary between the two runs, and why it bears on this interface.
- [Design status](../../design-status.md) — what is decided, what is proposed, and
  what the corpus leaves open.
