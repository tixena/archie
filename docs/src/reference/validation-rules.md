# Validation rules

**Status:** Mixed — see markers.

This page is the rule catalog from archie's root design document: the checks a
validation run performs, the level each one reports at, the message it prints,
and whether it can be repaired without a person deciding anything.

Read the next section before the catalog. The catalog is the oldest layer of the
design and it is scheduled to be replaced.

## The catalog is deferred, not current

**Decided.** The rule catalog encodes an earlier, hard-coded model and waits to
be reworked. The root design carries a deferral note at the head of its
validation-rules section, dated 2026-08-16 and written after Eduardo Burgos's
schema clarifications: the rules below, the `[rules]` and `[resource_types]`
blocks in the configuration, and the list of what the repair command can fix all
assume a global rule that every file must have frontmatter with the inbox as the
lone exception, `type` and `created` as fixed requirements, and a fixed list of
resource types. Under the direction Eduardo set — schemas rule frontmatter and
structure, resource types belong to remargin and are defined per vault, and
shapes can be declared for any document or sub-part in a strict or a loose mode —
each of these rules becomes a consequence of what the schemas say rather than a
rule in its own right.

**Decided.** The rework waits for the shape and schema language to be designed.
When an agent asked on 2026-07-27 whether to rework these sections then or later,
Eduardo answered on 2026-08-16: "I don't know but as long as you can keep it in
mind I feel safe." The deferral note is where it is kept. See
[Shape language](shape-language.md).

**Proposed.** The command that runs these rules is spelled `check` in the root
design and `lint` in the list of commands Eduardo wrote on 2026-08-30, where
`lint` stands in the same position. Which spelling survives is not settled; see
[archie check](cli/check.md).

## Rule categories

The root design groups the rules as follows.

| Category | Rules |
|---|---|
| Frontmatter | `frontmatter-required`, `frontmatter-valid` |
| Properties | `type-required`, `type-valid`, `created-required`, `inheritance` |
| Structure | `index-required`, `allowed-types` |
| Archive | `archive-placement` |
| Assets | `orphaned-assets` |
| Tasks | `embedded-tasks` |
| Inbox | `inbox-raw` |

## The rules at a glance

Levels are read from the message each rule prints. Repairability is read from the
list of what the repair command can and cannot fix, and from the repair lines the
root design attaches to individual rules.

| Rule | Level | What it checks | Repairable |
|---|---|---|---|
| `frontmatter-required` | Error | Every markdown file has frontmatter, except the inbox when raw inbox files are allowed | Not listed as repairable |
| `frontmatter-valid` | Error | The frontmatter parses | No |
| `type-required` | Error | The file carries a `type` property | No — a person decides |
| `type-valid` | Error | The `type` value is one of the defined resource types | No — a person decides |
| `created-required` | Error | The file carries a `created` date | Yes, from the file's modification time |
| `inheritance` | Error | The file carries the properties declared in the `inherits:` block of a parent `index.md` | Yes, by adding the missing properties |
| `index-required` | Error | Every folder that is not ignored has an `index.md` | Yes, by creating it with the inherited properties |
| `allowed-types` | Error | A file's type is permitted by the `allows.types` list in its folder's `index.md` | No |
| `archive-placement` | Error | A file marked `archived: true` lives under an `archive/` folder | Yes, by moving the file |
| `orphaned-assets` | Warning | Every asset is referenced by at least one markdown file | No |
| `embedded-tasks` | Warning | A note does not hold several independent checkbox items | No |
| `inbox-raw` | Information | Names inbox files that still have no frontmatter | No — the inbox command processes them |

One disagreement inside the corpus, recorded here as an observation and not
resolved by this book: the repair command's list of what it can fix does not
include creating frontmatter, while the sample JSON output in the root design
shows a `frontmatter-required` error carrying `"fixable": true`. The corpus does
not say which is right.

## The rules in full

The message under each rule is the message the root design specifies.

### `frontmatter-required`

Every markdown file must have YAML frontmatter, except the inbox when raw inbox
files are allowed by the configuration.

```
ERROR: File has no frontmatter
  → src/notes/random.md
```

### `frontmatter-valid`

The YAML frontmatter must be syntactically valid.

```
ERROR: Invalid YAML in frontmatter
  → src/notes/broken.md
  Line 3: expected ':' but found '-'
```

### `type-required`

Every file must have a `type` property.

```
ERROR: Missing required property 'type'
  → src/tixena/notes/meeting.md
```

### `type-valid`

The `type` value must be one of the defined resource types.

```
ERROR: Invalid type 'meeting'
  → src/notes/standup.md
  Valid types: index, person, organization, project, task, proposal,
               contract, note, snippet, workaround, walkthrough,
               research, reference, log, event
```

The list of types in that message is the fixed list from the configuration, which
is the part of this catalog Eduardo superseded on 2026-08-31 when he moved
resource types to remargin. See [Configuration](config.md).

### `created-required`

Every file must have a `created` date property. The root design marks it as
repairable from the file's modification time.

```
ERROR: Missing required property 'created'
  → src/tixena/projects/new.md

FIX: Can auto-add using file modification time
```

### `inheritance`

A file must carry the properties declared in the `inherits:` block of a parent
folder's `index.md`. The root design marks it as repairable by adding the missing
properties.

```
ERROR: Missing inherited property 'area'
  → src/tixena/people/team/dylan.md
  Expected: area: tixena
  Defined in: src/tixena/index.md
```

The mechanism this rule enforces is parked. Eduardo said on 2026-08-16 that
something like it is wanted but that he is not convinced the `inherits:` block is
the way, and asked to discuss other aspects first. See
[Inheritance](../concepts/inheritance.md).

### `index-required`

Every folder that is not ignored must have an `index.md`. The root design marks
it as repairable by creating the file with the inherited properties.

```
ERROR: Folder missing index.md
  → src/tixena/projects/new_project/
```

### `allowed-types`

If a folder's `index.md` declares an `allows.types` list, the files in that folder
must carry one of those types.

```
ERROR: Type 'project' not allowed in this folder
  → src/tixena/people/team/some_project.md
  Allowed types: [person]
  Defined in: src/tixena/people/team/index.md
```

### `archive-placement`

A file whose configured archive property is set must live under the archive
folder. The root design marks it as repairable by moving the file.

```
ERROR: File has archived=true but not in archive/
  → src/tixena/projects/old_project.md

FIX: Can move to src/tixena/archive/projects/old_project.md
```

### `orphaned-assets`

An asset — an image, a PDF, and the other extensions the configuration lists —
should be referenced by at least one markdown file.

```
WARN: Asset not referenced by any markdown file
  → src/tixena/assets/old_screenshot.png
```

The root design notes this rule can be slow on large corpora. Eduardo widened the
idea behind it on 2026-07-26: links are protected and orphaned files are
detected, and whether an orphaned file is then removed is to be elaborated. That
widening is recorded in the root design's purpose section and nowhere else.

### `embedded-tasks`

A note holding several independent checkbox items should be split into task
files. How many items trigger the warning is a configuration key.

```
WARN: File contains 5 independent task items
  → src/prodoctivity/notes/planning.md

  Consider splitting into individual task files.
```

The rule follows from a statement Eduardo made on 2026-07-26: every task is its
own item, and in this philosophy an item is one markdown document.

### `inbox-raw`

Information only: an inbox file without frontmatter still needs processing.

```
INFO: Inbox file needs processing (no frontmatter)
  → src/inbox/quick_note.md

  Run 'archie inbox' to classify
```

The command it points at is one of the features whose fate is an open question in
the program's vision blueprint; see [archie inbox](cli/inbox.md).

## Rules for knowledge patterns

**Proposed.** The root design adds three more rules, in a section of its own, for
the observation and relation notation described under
[Knowledge patterns](../concepts/knowledge-patterns.md). Unlike the rules above,
each of these three has its level set by a configuration key rather than fixed.

| Rule | Default level | What it checks |
|---|---|---|
| `observation-format` | Warning | An observation line follows the `- [category] content` form |
| `relation-target-exists` | Warning | A relation's target exists as a file in the corpus |
| `relation-type-valid` | Information | A relation's type comes from the known set, or from the additions in the configuration |

```
WARN: Invalid observation format
  → src/projects/nexus.md:15
  Found: - [bad format
  Expected: - [category] content
```

```
WARN: Relation target not found
  → src/projects/nexus.md:22
  - depends_on [[Nonexistent Service]]
  No file found matching "Nonexistent Service"
```

```
INFO: Unknown relation type 'foobar'
  → src/projects/nexus.md:23
  Consider: depends_on, blocks, owned_by, related_to, part_of, ...
```

## Choosing which rules run, and what a run returns

**Proposed.** Three controls exist in the root design, all of them part of the
same deferred layer.

- Each rule can be turned on or off in the configuration file, and the
  configuration is where the rule-specific settings live. See
  [Configuration](config.md).
- The validation command takes a rule filter, so that a run can be narrowed to
  named rules.
- The validation command takes a level at which the run fails, chosen from error,
  warning, or any, with error as the default. The level chosen is what decides the
  process exit code; see [Exit codes](exit-codes.md).

**Proposed.** The root design states as a principle that archie never modifies a
file without an explicit repair flag, which is why the table above separates what
a rule detects from what a repair can do. See [archie fix](cli/fix.md).

## What the corpus does not say

- **What a shape violation reports.** Every rule above is a fixed check with a
  fixed message. What the linter prints when a document fails a user-declared
  shape is not written down anywhere.
- **Which of these rules survives.** The deferral note says each becomes a
  consequence of what the schemas say; it does not say which rule names, levels,
  or messages remain.
- **Where custom rules fit.** The root design's plugin decision sketches rules
  declared in the configuration and validation delegated to shell scripts, as
  stages of a proposed approach rather than as a settled mechanism. See
  [Configuration](config.md).
