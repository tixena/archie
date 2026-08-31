# Output formats

**Status:** Proposed — see markers.

**Proposed.** A validation run can print its result in three formats: text, which
is the default and is meant for a person at a terminal; JSON, meant for another
program; and markdown, meant to be committed or pasted as a report. The format is
chosen with the `--format` option on the validation command, and its default comes
from the `[output]` table of the configuration file. See
[Configuration](config.md).

The samples below are the ones the root design document specifies, reproduced as
it writes them. They are the only place in the design corpus where the output of a
command is written out, which is why this is the only page in this book that shows
one.

Two notes before the samples. The root design's principle for the JSON format is
that archie is friendly to continuous integration: exit codes and JSON output for
automation. And the literal version strings in the samples — `archie v0.1.0` in
the text sample and `"version": "0.1.0"` in the JSON sample — are example banner
text; the corpus records, on 2026-08-31, that they can become neutral placeholders
on Eduardo Burgos's word, and he has not given it.

## Text

The default. It groups findings by level, prints the path and the message for
each, and closes with a count per level and a count of the files that passed.

```
archie v0.1.0

Scanning src/...
Found 127 markdown files, 45 assets

ERRORS (3):
  ✗ src/notes/random.md
    Missing frontmatter

  ✗ src/tixena/new/
    Missing index.md

  ✗ src/tixena/projects/old.md
    archived=true but not in archive/

WARNINGS (2):
  ⚠ src/prodoctivity/notes/planning.md
    Contains 5 embedded task items

  ⚠ src/tixena/assets/screenshot.png
    Not referenced by any file

INFO (4):
  ○ src/inbox/note1.md - Needs processing
  ○ src/inbox/note2.md - Needs processing
  ○ src/inbox/note3.md - Needs processing
  ○ src/inbox/note4.md - Needs processing

─────────────────────────────────────
Summary: 3 errors, 2 warnings, 4 info
         122 files OK
```

Color is on by default and can be turned off, either by the `color` key in the
configuration or by the `NO_COLOR` environment variable.

## JSON

One object per run. It carries the tool version, the time of the run, the corpus
root, the counts of what was scanned, the findings grouped into errors, warnings,
and information, and a summary. Each finding names the rule that produced it, the
path it concerns, its message, and whether it can be repaired.

```json
{
  "version": "0.1.0",
  "timestamp": "2025-12-23T10:30:00Z",
  "vault_root": "src",
  "files_scanned": 127,
  "assets_scanned": 45,
  "results": {
    "errors": [
      {
        "rule": "frontmatter-required",
        "path": "src/notes/random.md",
        "message": "Missing frontmatter",
        "fixable": true
      }
    ],
    "warnings": [],
    "info": []
  },
  "summary": {
    "errors": 3,
    "warnings": 2,
    "info": 4,
    "ok": 122
  }
}
```

The `fixable` field on the sample finding disagrees with the list of what the
repair command can fix, which does not include creating frontmatter. The
disagreement is recorded on [Validation rules](validation-rules.md); the corpus
does not resolve it.

## Markdown

A report document: a summary table of counts, then a section per level in which
findings are grouped by their message.

```markdown
# Archie Report

Generated: 2025-12-23 10:30:00

## Summary

| Level | Count |
|-------|-------|
| Errors | 3 |
| Warnings | 2 |
| Info | 4 |
| OK | 122 |

## Errors

### Missing frontmatter
- `src/notes/random.md`

### Missing index.md
- `src/tixena/new/`

...
```

The root design ends the sample with an ellipsis, so the shape of the warning and
information sections is not written down.

## What the corpus does not specify

- **Output for anything but a validation run.** These three formats belong to the
  command that validates a corpus. What `archie ls`, `archie get`, `archie link`,
  `archie rm`, or `archie init` print is written nowhere. Neither is whether
  `--format` applies to them at all.
- **Output for a query.** The grammar document's strawman proposes one JSON value
  per line as the default, with markdown available for fragments, and lists what a
  projection produces as one of its open questions: markdown fragments,
  frontmatter values as YAML or as JSON, file paths, or a mix chosen per clause.
  Nothing is settled. See [Query grammar](query-grammar.md).
- **Output for an export.** The root design's export command has a JSON shape of
  its own — each file's frontmatter, its observations, its relations, and a graph
  of nodes and edges — and that command is one of the features whose fate is an
  open question in the program's vision blueprint. See
  [archie export](cli/export.md).
- **Whether the text format is stable.** Nothing says whether the text output is
  a contract another program may parse, or a rendering that may change.

## The Model Context Protocol server

**Decided, stated outside the design corpus.** archie exposes a Model Context
Protocol server, just like remargin. Eduardo said so in chat on 2026-08-31; no
document in the design corpus records it, and the gap is listed on the
[Design status](../design-status.md) page.

Nothing states what such a server returns, so nothing about its payloads appears
on this page. remargin's own conventions — a compact payload whose repeated rows
are positional and named once by a list of column names, and a total count that is
always exact — are what archie's server is expected to mirror. See
[Model Context Protocol server](mcp/index.md).

## Where this leads

- [Exit codes](exit-codes.md) — what a run returns to the shell.
- [Validation rules](validation-rules.md) — what produces the findings shown here.
- [Run archie in continuous integration](../guides/run-in-ci.md) — the JSON format
  in use.
