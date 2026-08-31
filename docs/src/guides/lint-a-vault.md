# Lint a vault

**Status:** Mixed — see markers.

Linting a corpus is one run: archie walks a tree of markdown files, applies its
rules, and prints what it found at three levels of severity. This guide covers
starting the run, reading its report, narrowing it, and deciding when it should
fail. It is the most complete guide in this book, because the validation run is
the most completely specified part of archie's design — and it carries one
qualification throughout, which is that the rules it applies are scheduled to be
replaced.

## Before you start

**Two spellings.** The root design document calls the command `check`. The
command list Eduardo Burgos wrote on 2026-08-30 has `lint` in the same position
and never says `check`. Every option below is written for `check`; no option is
recorded for `lint`. No document reconciles the two names. See
[archie check](../reference/cli/check.md).

**Root design.** The run never modifies a file. Reporting and repairing are
separate acts, and only the repair run writes.

**Deferred.** The rules the run applies encode an earlier model — frontmatter
required everywhere with the inbox as the lone exception, `type` and `created` as
fixed requirements, a fixed list of resource types — and Eduardo recorded on
2026-08-16 that each of them becomes a consequence of what the schemas say once
the schema language is designed. The catalog is what archie's design currently
specifies and is not what it will settle on. See
[Validation rules](../reference/validation-rules.md).

## Step 1: choose what to scan

```console
$ archie check
$ archie check src/tixena/
```

With no path, the run scans the corpus root named in the configuration file. With
a path, it scans that path. Folders named in the configuration's ignore list are
skipped in both cases; the defaults skip Obsidian's own folder, the assets folder,
and the templates folder. See [Configuration](../reference/config.md).

## Step 2: read the report

The default output is text, grouped by severity. This is the report the root
design document specifies, with its opening banner line omitted because this book
carries no release framing:

```console
$ archie check
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

Read it in three parts.

- **Errors** are documents that break a rule the design treats as a violation: no
  frontmatter, frontmatter that does not parse, a missing or unrecognized `type`,
  a missing `created` date, a missing inherited property, a folder with no index
  file, a file whose type its folder does not allow, and a file marked as archived
  that is not in an archive folder.
- **Warnings** are two findings that ask for a judgment rather than state a
  breakage: an asset no markdown file references, and a note holding several
  independent checkbox items that the design would rather see split into task
  files. The threshold for the second is a configuration key.
- **Information** is the inbox: files that still have no frontmatter and are
  waiting to be classified.

Every rule, with the message the design gives it and whether a repair can address
it, is on the [Validation rules](../reference/validation-rules.md) page.

Two other formats exist for the same run: JavaScript Object Notation (JSON) for
another program to read, and markdown for a report to commit or paste. Select one
with `--format json` or `--format markdown`, or set the default in the
configuration file's `[output]` table. Both are shown in full on
[Output formats](../reference/output-formats.md), and JSON is the one to reach for
in an automated pipeline — see
[Run archie in continuous integration](run-in-ci.md).

## Step 3: narrow the run

```console
$ archie check --rule frontmatter --rule type
$ archie check src/tixena/ --ignore templates
```

`--rule` restricts the run to named rules and may be given more than once.
`--ignore` adds patterns to skip, on top of the configuration file's ignore list;
what a pattern may contain is not stated anywhere, and the ignore list in the
configuration holds plain folder names. Rules can also be turned off one at a time
in that file, which is the place for a decision that holds for every run rather
than for one.

One ambiguity to know about: the root design's own example passes
`--rule frontmatter --rule type`, and those two words are category names in the
rule catalog rather than rule names — the catalog's rule names are
`frontmatter-required`, `frontmatter-valid`, `type-required`, and `type-valid`.
Whether the option takes a name, a category, or either is not stated anywhere.

## Step 4: decide when the run should fail

```console
$ archie check --fail-on warning
```

`--fail-on` takes `error`, `warning`, or `any`, and defaults to `error`. It
decides the process exit status and nothing else: the same findings are printed
either way.

| Exit code | What it means |
|---|---|
| 0 | The run finished and nothing reached the configured level. |
| 1 | Validation errors were found. |
| 2 | No errors, but warnings were found and the run was set to fail on warnings. |
| 3 | A configuration error. |
| 4 | A file system error. |

The table is the root design's, and it also carries a fifth code for an error
from a language-model provider, which a validation run cannot produce because it
uses no model. What the run returns when the level is `any` and the only findings
are informational is not stated. See [Exit codes](../reference/exit-codes.md).

> **Suggestion (documentation author):** the design gives no advice on how to
> adopt the linter on a corpus that has never been checked, and a reader needs
> one, so here is this book's — offered as a habit and not as design. Run first
> with the default level, so that only errors are fatal; clear the errors; then
> move to `--fail-on warning` and keep it there, so that new warnings cannot
> accumulate quietly. Eduardo would have to decide whether an adoption path like
> that is something archie documents and supports — with, for instance, a way to
> record which existing findings are accepted so that a stricter level can be
> turned on before they are all resolved — or whether the two levels are the whole
> of the story and the sequence is one person's practice.

## Step 5: repair what can be repaired

```console
$ archie fix --dry-run
$ archie fix src/tixena/ --rule inheritance
```

The repair run addresses the findings that need no human decision: adding a
missing `created` date from the file's modification time, adding missing inherited
properties, creating missing folder index files, and moving a file marked as
archived into the archive folder. It refuses the three findings that need a
person: a missing `type`, an invalid `type`, and the classification of an inbox
file.

Two qualifications. The repair list is inside the same deferral as the rules, so
what is repairable will follow from the shapes rather than from that list. And
the repair command appears in the root design and not in Eduardo's own command
list, which places it among the features whose fate the program's vision blueprint
holds open. See [archie fix](../reference/cli/fix.md).

## What this run becomes

**Proposed (stated by Eduardo on 2026-08-25).** The rules stop being a fixed
catalog. Eduardo placed a linter with user-specified shapes in archie that day,
with a strict mode that rejects content outside the declared shape and a loose
mode that lets it through; each rule above then becomes a consequence of what a
shape says. What the linter prints when a document fails a shape is not written
down anywhere, so this guide's step 2 describes the report archie's design has
today and not the one it is heading for. See [Define a shape](define-a-shape.md)
.

## Related pages

- [archie check](../reference/cli/check.md) — the full option table and the two
  spellings.
- [Validation rules](../reference/validation-rules.md) — every rule, its level,
  its message, and whether it is repairable.
- [Output formats](../reference/output-formats.md) — the text, JSON, and markdown
  forms of this report.
- [Exit codes](../reference/exit-codes.md) — the code table and what is unstated
  about it.
- [Run archie in continuous integration](run-in-ci.md) — the same run as a build
  step.
- [Configuration](../reference/config.md) — the corpus root, the ignore list, and
  the per-rule switches.
- [A first session with archie](../tutorial/first-session.md) — the same steps in
  one pass, front to back.
