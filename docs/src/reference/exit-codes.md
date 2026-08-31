# Exit codes

**Status:** Proposed — see markers.

**Proposed.** The root design document gives archie six exit codes. They appear in
its section on error handling and nowhere else in the design corpus; no statement
of Eduardo Burgos's confirms or changes them.

| Code | Meaning |
|---|---|
| 0 | Success, no issues |
| 1 | Validation errors found |
| 2 | Warnings only, when the run is set to fail on warnings |
| 3 | Configuration error |
| 4 | File system error |
| 5 | Language-model provider error |

**Proposed.** The reason the codes exist is stated as a design principle: archie
is friendly to continuous integration, and exit codes together with JSON output
are what makes it so. See [Run archie in continuous integration](../guides/run-in-ci.md).

## What decides between 0, 1, and 2

**Proposed.** The validation command takes an option that sets the level at which
a run fails, chosen from error, warning, or any, with error as the default. Code 1
is what a run returns when it found errors and the level says errors are fatal.
Code 2 is what a run returns when it found no errors, found warnings, and the level
says warnings are fatal.

The corpus does not say what a run returns when the level is set to any and the
run found only information-level findings, nor whether findings that exist but do
not reach the configured level change the code at all. The table's entry for code 2
names only the warning setting.

## The other three codes

**Proposed.** Code 3 covers a configuration error, code 4 a file system error, and
code 5 an error from the provider of the language model. Nothing in the corpus
enumerates what counts as each — whether a missing configuration file is a
configuration error or a file system error, for instance.

Code 5 belongs to the feature that classifies inbox captures with a language model.
That feature's fate is one of the program's open questions: the vision blueprint
asks whether archie's application features live inside archie, inside a separate
application built on the engine, or are dropped. If it leaves, the code has nothing
to report. See [archie inbox](cli/inbox.md).

## What an error message says

**Proposed.** The root design pairs the code table with three requirements for an
error message — that it is clear about what is wrong, specific about where, and
actionable — and one illustration of a message that meets them:

```console
$ archie check
ERROR: Invalid type 'meeting'
  → src/notes/standup.md:2

  The type 'meeting' is not recognized. Did you mean 'note'?
  Valid types: index, person, organization, project, task, ...

  Hint: Use 'type: note' with 'tags: [meeting]' instead.
```

Two things separate it from the same rule's message in the catalog, which names
only the file, the offending value, and the valid types. It carries a line number,
and it carries a suggested correction — a nearest known type, and a way to keep
the rejected word as a tag instead. Neither is written into any rule's own
message. The list of valid types it prints is the fixed list Eduardo superseded on
2026-08-30 when he moved resource types to remargin. See
[Validation rules](validation-rules.md).

**Not in the corpus.** Whether the suggestion behavior is required of every rule
or belongs to this one message, and how a nearest known value is chosen.

## What the corpus does not specify

- **Codes for the commands Eduardo named.** The table was written for the
  validation command and its neighbours in the root design. The list of commands
  Eduardo wrote on 2026-08-30 — listing, retrieving, linting, linking,
  initializing, searching, writing, removing, querying — has no exit codes attached
  anywhere. See the [Command-line interface](cli/index.md) reference.
- **What a query that matched nothing returns.** Nothing says whether an empty
  result is success or a distinct code.
- **What a refused transaction returns.** The structural operations captured in
  the corpus move nothing at all if any entry is refused; what the process then
  returns is not written down. See
  [Structural operations](cli/structural-operations.md).
- **Whether a repair run has codes of its own.** The repair command is documented
  with a dry run, a rule filter, and an interactive mode, and with no exit codes.

One neighbouring behavior is recorded in the corpus, for a program that is not
archie: the `link` subcommand of Eduardo's `aa_tool` project exits non-zero from a
dry run when edits are pending, so that a check in an automated pipeline fails
while nothing is written. archie's `link` generalizes that implementation, and the
corpus does not say whether it inherits that convention. See
[archie link](cli/link.md).

> **Suggestion (documentation author):** the six codes above cover a corpus
> validator and nothing else, while the commands Eduardo actually listed are a
> read, write, and search surface. Two decisions would settle the rest, and both
> are his. First, whether every command shares one table — so that 3 is always a
> configuration error and 4 is always a file system error, whichever verb ran —
> or whether each verb defines its own. Second, whether "found nothing" is a
> distinct code, the way `grep` returns 1 for no match, or plain success; the
> answer decides whether `archie query` can be used as a test in a shell script
> without inspecting its output. Nothing here has standing until he decides it.

## Where this leads

- [Output formats](output-formats.md) — what a run prints alongside its code.
- [Validation rules](validation-rules.md) — what produces the errors and warnings
  the codes report.
- [Configuration](config.md) — where the default output format and the enabled
  rules come from.
