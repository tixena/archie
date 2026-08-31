# Exit codes

archie returns one of six exit codes.

| Code | Meaning |
|---|---|
| 0 | Success, no issues |
| 1 | Validation errors found |
| 2 | Warnings only, when the run is set to fail on warnings |
| 3 | Configuration error |
| 4 | File system error |
| 5 | Language-model provider error |

Exit codes together with [JSON output](output-formats.md) are what make archie
friendly to [continuous integration](../guides/run-in-ci.md).

## What decides between 0, 1, and 2

The validation command takes the level at which a run fails — error, warning, or
any, with error as the default. Code 1 is what a run returns when it found errors
and errors are fatal. Code 2 is what it returns when it found no errors, found
warnings, and warnings are fatal.

*What a run returns when the level is set to any and only information-level
findings were made, and whether findings below the configured level change the
code at all, are not yet specified.*

## The other three codes

Code 3 covers an error in the [configuration](config.md), code 4 a file system
error, and code 5 an error from the language-model provider. Code 5 belongs to
the classification of inbox captures; see [archie inbox](cli/inbox.md).

*What counts as each — whether a missing configuration file is a configuration
error or a file system error, for instance — is not yet specified.*

## What an error message says

An error message is clear about what is wrong, specific about where, and
actionable.

```console
$ archie check
ERROR: Invalid type 'meeting'
  → src/notes/standup.md:2

  The type 'meeting' is not recognized. Did you mean 'note'?
  Valid types: index, person, organization, project, task, ...

  Hint: Use 'type: note' with 'tags: [meeting]' instead.
```

Two things separate this from the same rule's message in the catalog, which names
only the file, the offending value, and the valid types: a line number, and a
suggested correction — a nearest known type, and a way to keep the rejected word
as a tag instead. See [Validation rules](validation-rules.md).

*Whether every rule offers a suggestion, and how a nearest known value is chosen,
are not yet specified.*

## Not yet specified

- Codes for the commands that read, write, and search rather than validate. The
  table is written in the language of a validation run. See the
  [Command-line interface](cli/index.md).
- Whether a query that matched nothing returns success or a code of its own.
- What a refused transaction returns. A structural operation moves nothing at all
  if any entry is refused; see
  [Structural operations](cli/structural-operations.md).
- Whether a repair run has codes of its own, and whether a
  [link](cli/link.md) dry run with pending edits exits non-zero.

See [Design status](../design-status.md) for the open questions around exit
codes.
