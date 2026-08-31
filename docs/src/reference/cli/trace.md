# archie trace

**Status:** Mixed — see markers.

`archie trace` prints the chain of inherited properties for one file: which
folder index files above it declare properties, what those properties are, and
whether the file carries them.

## Status of the command

**Root-design-only, fate open.** `archie trace` appears in the root design
document and nowhere in Eduardo Burgos's command list of 2026-08-30, so it falls
under the vision blueprint's open question about what becomes of archie's
features for maintaining a personal corpus.

**Parked, in addition.** The mechanism this command displays is itself under
reconsideration. The root design records Eduardo's statement of 2026-08-16: he
wants something like property inheritance from folder index files, but whether
the `inherits:` block in a folder's `index.md` is the way is not settled, and the
question is parked until other aspects are discussed. No alternative is proposed
anywhere. A command whose whole job is to display that mechanism cannot be more
settled than the mechanism.

## Synopsis

```console
$ archie trace <FILE>
```

## Arguments

| Argument | What the corpus says |
|---|---|
| `<FILE>` | The file whose inheritance chain to print. Required. The root design's example passes a path to a person file inside a nested folder. |

## Options

The root design states none for this command.

## Behavior

**Root design.** The run walks from the corpus root down to the file, and for
each folder on the way prints that folder's index file and the properties its
`inherits:` block declares — noting which of those the folder took from its own
parent and which it adds at its level — and then prints, for the file itself, the
properties it must carry as a result and the properties it actually carries, so
that the two can be compared.

**Root design.** How inheritance accumulates, which is what the printed chain
shows: a folder's `index.md` declares an `inherits:` block whose properties apply
to every file in that folder and in all of its subfolders; each level adds to what
it inherited; a child file may add properties of its own but never drops an
inherited one. The worked example in the corpus has one folder declare an area,
a folder further down inherit that area and add a role, and a file two levels
below required to carry both.

**Not stated.** There is no override mechanism written down. The workspace
analysis says the tool validates that a file's values match the inherited ones,
which means a child declaring a different value for an inherited property fails
validation, but no document states that as a rule of its own.

## Examples

**Root design.** The design specifies this command's output, so it is shown. The
command line and the report below are both the root design's own, an indented
tree with one block per folder index file, ending with two lines for the file —
what it must carry, and what it does carry:

```console
$ archie trace src/tixena/people/team/dylan_minaya.md
Inheritance chain for: src/tixena/people/team/dylan_minaya.md

src/index.md
  └─ (no inherits block)

src/tixena/index.md
  └─ inherits:
       area: tixena

src/tixena/people/index.md
  └─ inherits:
       area: tixena (from parent)

src/tixena/people/team/index.md
  └─ inherits:
       area: tixena (from parent)
       role: employee

dylan_minaya.md
  └─ MUST have: area=tixena, role=employee
  └─ HAS: area=tixena, role=employee ✓
```

The check mark on the last line is the report saying the two agree. Nothing
states what the line reads when they do not, and nothing states whether the
report is offered in any other format: the three formats of the validation run
are written for that run alone.

## Exit codes

The root design document's exit-code table does not mention `archie trace`. In
particular, nothing states whether the command exits with a non-zero status when
the file is missing a property it must carry, or whether it only reports. See
[Exit codes](../exit-codes.md).

## Related pages

- [Inheritance](../../concepts/inheritance.md) — the mechanism this command
  displays, and why it is parked.
- [archie check](check.md) — the `inheritance` rule that reports a missing
  inherited property as a finding.
- [archie init](init.md) — the command that creates a missing folder index file.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
