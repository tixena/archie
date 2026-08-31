# archie fix

`archie fix` repairs the findings a validation run reports that can be repaired
without a person deciding anything. It is the only command that changes files
without being told what content to write.

## Synopsis

```console
$ archie fix [PATH]
```

`[PATH]` is the path to repair; it is optional.

## Options

| Option | Effect |
|---|---|
| `--dry-run` | Show what would be repaired without changing any file. |
| `--rule <RULE>` | Repair only the findings of the named rule or rules. |
| `--interactive` | Ask before each repair. |

## What it repairs

- Add a missing `created` date, taken from the file's modification time.
- Add missing properties that a parent folder's index file declares as inherited.
- Create missing `index.md` files.
- Move a file carrying `archived: true` into the archive folder.

Three findings need a person to decide and are never repaired: a missing `type`
property, an invalid `type` value, and the classification of an inbox file.

## Behavior

Files are never modified without an explicit repair. [`archie check`](check.md)
reports; only this command writes.

## Examples

```console
$ archie fix --dry-run
$ archie fix src/work/ --rule inheritance
```

## Not yet specified

- What `--dry-run` prints, whether `--interactive` and `--dry-run` may be
  combined, and what the prompt offers in interactive mode.
- Whether a repair run validates first, whether it reports what it repaired and
  in which format, and whether a failed repair halts the run.
- Whether a repair is a [change object](../change-objects.md), replayable and
  unappliable like any other write.
- Whether moving an archived file rewrites the inbound links to it, as
  [structural operations](structural-operations.md) do.
- Which repairs follow from a schema once the
  [shape language](../shape-language.md) is defined.
- Output, and the exit codes for a run that repaired everything, one that
  repaired some findings and left others, and a repair that failed.

See [Design status](../../design-status.md) for the open questions here.

## Related

- [archie check](check.md) — the validation run this repairs the findings of.
- [Validation rules](../validation-rules.md) — which rules are repairable.
- [Shape language](../shape-language.md) — the schemas that decide what is
  repairable.
- [Change objects](../change-objects.md) — the transactional model.
- [Command-line interface](index.md) — global options and the command table.
