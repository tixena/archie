# archie fix

**Status:** Mixed — see markers.

`archie fix` repairs the findings a validation run reports that can be repaired
without a person deciding anything. It is the only command in the corpus that
changes files without being asked to write specific content.

## Status of the command

**Root-design-only, fate open.** `archie fix` appears in the root design
document's command section and nowhere in Eduardo Burgos's command list of
2026-08-30. The vision blueprint holds open what becomes of archie's features for
maintaining a personal corpus: whether they live inside archie, inside a separate
corpus-focused application built on the engine, or are dropped. Nothing in the
corpus answers that for this command.

**Deferred, in addition.** The list of what `archie fix` can and cannot repair is
covered by the deferral Eduardo recorded on 2026-08-16: it encodes the earlier
hard-coded rule model, and under the schema-driven direction each repair becomes a
consequence of what the schemas say. The rework waits for the schema language to
be designed.

## Synopsis

```console
$ archie fix [PATH]
```

## Arguments

| Argument | What the corpus says |
|---|---|
| `[PATH]` | The path to repair. Optional. The root design does not restate a default for this command; the validation run it mirrors takes its default from the configuration file. |

## Options

From the root design document's command section.

| Option | What the corpus says |
|---|---|
| `--dry-run` | Show what would be repaired without changing any file. |
| `--rule <RULE>` | Repair only the findings of the named rule or rules. |
| `--interactive` | Ask before each repair. |

The corpus does not say what `--dry-run` prints, whether `--interactive` and
`--dry-run` may be combined, or what the prompt offers in interactive mode.

## What it repairs

**Root design, deferred.** Four repairs are listed as possible:

- Add a missing `created` date, taken from the file's modification time.
- Add missing properties that a parent folder's index file declares as inherited.
- Create missing `index.md` files.
- Move a file carrying `archived: true` into the archive folder.

Three findings are listed as needing a person to decide, and are therefore not
repaired: a missing `type` property, an invalid `type` value, and the
classification of an inbox file.

Read that split against the deferral above. The `created` and `type` properties
are fixed requirements only in the older model; once shapes declare what a
document must carry, what is repairable follows from the shape rather than from
this list.

## Behavior

**Root design.** Files are never modified without an explicit repair. The
principle is stated in the root design document as "never modify files without
explicit `--fix` flag" — a flag there, and a separate command here, in the
command section of the same document. The two spellings are not reconciled
anywhere; what both say is that the validation run reports and only a repair
writes.

The corpus does not say whether a repair run validates first and then repairs,
whether it reports what it repaired and in which format, or whether a repair that
fails halts the run.

**Related but unstated.** Eduardo's transactional change model, stated on
2026-08-25, makes every change through remargin an object that can be replayed to
a byte-identical result and unapplied in reverse. No document says whether the
repairs above are such change objects. Moving an archived file into the archive
folder is also, read alongside the structural-operations capture, a move — and
moves in that capture are transactions that rewrite every inbound link. The root
design's repair predates that capture and says nothing about links.

## Examples

```console
$ archie fix --dry-run
$ archie fix src/tixena/ --rule inheritance
```

Both are the root design's option set applied to its own spelling; the document
gives no worked example for this command, and specifies no output, so none is
shown.

## Exit codes

The root design document's exit-code table does not distinguish a repair run from
a validation run. What a repair run returns when it repaired everything, when it
repaired some findings and left others, or when a repair failed, is not stated.
See [Exit codes](../exit-codes.md).

## Related pages

- [archie check](check.md) — the validation run whose findings this command
  repairs.
- [Validation rules](../validation-rules.md) — each rule, including which ones
  the root design marks repairable.
- [Shape language](../shape-language.md) — the schemas that are to decide what is
  repairable.
- [Change objects](../change-objects.md) — the transactional model, and what it
  does not yet say about repairs.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
