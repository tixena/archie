# archie rm

**Status:** Proposed — see markers.

`archie rm` removes a file or a folder from a corpus. The corpus names the
command and describes no behavior for it.

## Synopsis

```console
$ archie rm folder/file.md
$ archie rm folder/
```

Both lines are Eduardo Burgos's, written on 2026-08-30 on the planning document.
They are the whole of what the design corpus says about `archie rm`.

## Status of the command

**Proposed.** Eduardo stated on 2026-08-30 that his command list is not written in
stone — the verbs and the syntax can change. That is recorded as decision G4 on
the grammar document.

## Arguments and options

| Part | What the corpus says |
|---|---|
| The path | A markdown file in the first example, a folder in the second. Required in both. |

No option is written down: no confirmation flag, no dry run, no recursion flag,
and no force flag.

## A spelling not to confuse this with

The planning document sketches a different `archie rm`: `archie rm priority
src/some_task.md` to remove a frontmatter field, and `archie rm tags "obsolete"
src/some_task.md` to remove a value from an array. Those lines were written by an
agent, not by Eduardo, and they belong to a set of field-editing commands that
the decided direction replaced: the query language itself drives modification.
This book does not revive them, and the `archie rm` documented on this page is
Eduardo's — removing a file or a folder, not a property.

## Behavior

The corpus describes none. Nothing states whether removing a folder removes its
contents, whether anything is asked before a removal, whether a removal can be
undone, or what happens to documents elsewhere in the corpus that link to what was
removed.

That last point is the one the corpus comes closest to addressing, in a document
about a different subject. The structural-operations capture records that
structural changes to a corpus — move, rename, archive — are archie's territory
precisely because they must keep links valid by construction rather than by
validating and repairing afterwards. Removal is not in that list, and the capture
does not mention it.

> **Suggestion (documentation author):** Eduardo would have to decide two things
> for this command to be specifiable. First, whether a removal is a change object
> under the transactional model he described on 2026-08-25 — that is, whether it
> can be unapplied to restore the removed content, which would make `archie rm`
> reversible in the way `archie link` and its inverse are. Second, what happens to
> inbound links to the removed resource: refuse the removal, remove and report the
> now-dangling links, or rewrite them. The structural-operations capture answers
> the equivalent question for moves and says nothing about removals.

## Examples

```console
$ archie rm folder/file.md
$ archie rm folder/
```

Eduardo's own examples. The design specifies no output for them, so none is
shown.

## Exit codes

The root design document's exit-code table is written in the language of the
validation run and does not mention `archie rm`. See
[Exit codes](../exit-codes.md).

## Related pages

- [Structural operations](structural-operations.md) — move, rename, archive, and
  merge as transactions that keep links valid.
- [Change objects](../change-objects.md) — the transactional model a removal
  would have to fit.
- [archie write](write.md) — the command that changes a document's contents.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
