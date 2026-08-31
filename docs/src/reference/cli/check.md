# archie check

**Status:** Mixed — see markers.

`archie check` is the validation run: it reads a corpus, applies rules to it, and
reports what it finds. It is the most fully specified command in the root design
document and, at the same time, the command whose rule set that same document
defers wholesale to a schema language that has not been designed.

## Two spellings

The root design document calls this command `check`. Eduardo Burgos's command
list of 2026-08-30 has `archie lint folder/` in the same position and never says
`check`. No document reconciles the two names. This page keeps `archie check` as
its title because that is the spelling the specified behavior is written under,
and records `lint` as Eduardo's own spelling for the same job.

The naming question is not idle. The root design document argues in its naming
section that "lint" undersells a tool that also repairs, classifies, imports, and
creates structure — but that argument was made about naming the program, which
was then called `vault-lint` and became `archie`, not about naming this command.
Under the direction Eduardo set on 2026-08-25, archie's own territory is the
linter and the schema engine, and the repairing and classifying features are the
ones whose fate is open.

## Status of the command

**Root-design-only, fate open.** `archie check` appears in the root design
document and not in Eduardo's list; `archie lint` appears in his list and not in
the root design. What archie's features for maintaining a personal corpus become
is one of the vision blueprint's open questions. The linter itself is not in
doubt — Eduardo placed "a linter with user-specified shapes" inside archie on
2026-08-25, and it is the first entry in his feature ledger — but its command-line
spelling and its option set are.

## Synopsis

```console
$ archie check [PATH]
$ archie lint folder/
```

The first line is the root design document's, the second is Eduardo's.

## Arguments

| Argument | What the corpus says |
|---|---|
| `[PATH]` | The path to validate. Optional; the default comes from the configuration file. |

## Options

Every option below is from the root design document's command section. No option
is stated for the `lint` spelling.

| Option | What the corpus says |
|---|---|
| `--format <FORMAT>` | Output format: `text`, `json`, or `markdown`. Default: `text`. |
| `--fail-on <LEVEL>` | Exit with a non-zero status on `error`, `warning`, or `any`. Default: `error`. |
| `--rule <RULE>` | Run only the named rule or rules. The example passes it more than once. |
| `--ignore <PATTERN>` | Additional patterns to ignore, beyond the ignore list in the configuration file. |

The corpus does not say whether `--rule` takes a rule name, a rule category, or
either: its own example passes `--rule frontmatter --rule type`, which are
category names in the rule catalog rather than rule names.

## The rules

**Deferred.** Before reading the catalog, read the deferral Eduardo recorded on
2026-08-16 at the top of the root design document's validation section. The rules
below, the rule and resource-type blocks in the configuration file, and the list
of what `archie fix` can repair all still encode an earlier, hard-coded model: a
global rule that frontmatter is required with the inbox as the lone exception, a
`type` property and a `created` property as fixed requirements, and a fixed list
of resource types. Under the schema-driven direction — schemas rule both
frontmatter and structure, resource types belong to remargin and are defined per
corpus, and shapes can be declared for any document or sub-part in a strict or a
loose mode — each of these rules becomes a consequence of what the schemas say.
The rework waits for the schema language to be designed. The catalog is recorded
so that it is not lost, not because it is settled.

| Rule | Category | What it reports |
|---|---|---|
| `frontmatter-required` | Frontmatter | A markdown file with no frontmatter block. An error. The inbox is exempt when the configuration allows raw files there. |
| `frontmatter-valid` | Frontmatter | A frontmatter block that is not syntactically valid, reported with the line at which parsing failed. An error. |
| `type-required` | Properties | A file with no `type` property. An error. |
| `type-valid` | Properties | A `type` value that is not one of the defined resource types. An error; the message lists the valid types. |
| `created-required` | Properties | A file with no `created` date property. An error, and repairable from the file's modification time. |
| `inheritance` | Properties | A file missing a property that a parent folder's index file declares under `inherits`. An error naming the expected value and the index file that declares it; repairable. |
| `index-required` | Structure | A folder with no `index.md`, other than an ignored one. An error; repairable by creating the file with the inherited properties. |
| `allowed-types` | Structure | A file whose `type` is not in the `allows.types` list declared by its folder's index file. An error naming the allowed types and the index file that declares them. |
| `archive-placement` | Archive | A file carrying `archived: true` that is not inside an archive folder. An error; repairable by moving it. |
| `orphaned-assets` | Assets | An asset — an image, a document in Portable Document Format, and so on — that no markdown file references. A warning. The configuration notes it can be slow on a large corpus. |
| `embedded-tasks` | Tasks | A note holding several independent checkbox items, which the design suggests splitting into task files. A warning, with a configurable threshold. |
| `inbox-raw` | Inbox | An inbox file with no frontmatter, which needs processing. Reported at the information level only. |

Each rule is described in full, with the message text the root design gives it, on
the [Validation rules](../validation-rules.md) page.

## Behavior

**Root design.** A run walks the configured corpus root, or the given path, skips
what the ignore list excludes, applies the enabled rules, and reports every
finding at one of three levels: error, warning, or information. Which rules run
is controlled by the configuration file, where every rule can be turned on or
off, and narrowed further by `--rule` on the command line. Whether the run's exit
status is non-zero is controlled by `--fail-on`.

**Root design.** The run never modifies a file. Non-destructiveness is one of the
root design's stated principles, in its words "never modify files without
explicit `--fix` flag" — written as a flag there, and spelled as a separate
command, [`archie fix`](fix.md), in the command section of the same document. The
two spellings are not reconciled anywhere; what both say is that reporting and
repairing are separate acts.

**Decided.** Eventually the rules come from schemas rather than from a fixed
catalog. Eduardo, 2026-08-25: archie holds a linter for which he can specify the
shape of any markdown document, or of a sub-part of one, and schemas have a
strict mode that rejects content outside the declared shape and a loose mode that
lets it through. He explicitly does not want a stripping mode, in his words
because he "wouldn't want to accept content and trim it." The shape language is
not designed; see [Shape language](../shape-language.md).

**Decided.** Resource types are not archie's. Eduardo, 2026-08-30: they are
business logic and belong to remargin, so archie holds only the means to enforce
them. What remargin needs from those means, stated in the same place: types
defined per corpus, because the type list in the workspace analysis was extracted
from Eduardo's own files and another corpus defines its own, and a type that can
extend another to add to it. This is recorded as a non-goal in the requirements
document, which makes the fixed type list behind `type-valid` an artifact of the
older model.

## Examples

Every example below is from the root design document.

```console
$ archie check
$ archie check src/tixena/
$ archie check --format json > report.json
$ archie check --rule frontmatter --rule type
```

The text output the root design specifies, with its opening banner line omitted
because this book carries no release framing:

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

The JavaScript Object Notation (JSON) and markdown forms of the same run are on
the [Output formats](../output-formats.md) page.

## Exit codes

The root design document's exit-code table is written for this command:

| Code | Meaning |
|---|---|
| 0 | Success, no issues. |
| 1 | Validation errors found. |
| 2 | Warnings only, when `--fail-on warning` was given. |
| 3 | Configuration error. |
| 4 | File system error. |
| 5 | Language-model provider error. |

Code 5 cannot arise from a validation run, which uses no language model. See
[Exit codes](../exit-codes.md).

## Related pages

- [Validation rules](../validation-rules.md) — every rule with its message.
- [Shape language](../shape-language.md) — the schemas that are to replace the
  fixed catalog.
- [archie fix](fix.md) — the repair run.
- [Output formats](../output-formats.md) — the three formats in full.
- [Lint a vault](../../guides/lint-a-vault.md) — the task-shaped guide.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
