# archie link

**Status:** Proposed — named by Eduardo on 2026-08-30; generalizes the `link`
subcommand of `aa_tool`; not yet designed. The feature ledger holds it as an
idea awaiting a feature discussion.

`archie link` turns mentions of a resource into links to that resource.

## Synopsis

```console
$ archie link folder/
```

The only spelling in the corpus. No option is defined for archie.

## Behavior

What is settled, ahead of the feature discussion that will define the rest:

- The behavior is "ALMOST the same" as `aa_tool link`
  (`~/src/tixena/aa_tool/src/link.rs`); every difference beyond the next bullet
  is reserved for that discussion.
- It is not restricted to people: any resource can be a link target. What counts
  as a resource, and which of its fields supply the strings to match, is
  remargin's to declare — resource types are remargin's, a stated non-goal for
  archie — and archie's to use.

*Source: planning document threads, 2026-08-30 and 2026-08-31 — Proposed.*

## The reference behavior

The contract of the implementation archie generalizes. These are facts about
`aa_tool`, adopted by archie only where the feature discussion says so:

- Wraps every collision-free mention of a resource's name or alias in a
  relative-path link; the display text is the matched text, unchanged.
- Deterministic: no heuristics. Targets come from identity records (`name` plus
  `aliases`, keyed by slug); the longest match wins, on Unicode word boundaries.
- Additive and reversible: a link only wraps text already present, so stripping
  every injected link reproduces the file byte for byte.
- Idempotent: text already inside a link is skipped; a second run changes
  nothing.
- Collision-guarded: a string claimed by two or more resources is never linked,
  and every such string is reported as excluded.
- Never rewrites frontmatter, fenced blocks (remargin comment blocks included),
  inline code, existing links and wikilinks, raw web addresses, a resource's own
  profile, or a document that declares aliases.
- Reports every edit as JSON: file, byte range, matched text, target slug,
  emitted path — plus the collision-excluded surfaces.
- Guarded: refuses a dirty git working tree unless `--allow-dirty`; `--dry-run`
  computes the edits without writing and exits non-zero when edits are pending;
  `--report <PATH>` writes the report. None of the three is written down for
  archie.
- Has an exact inverse: `unlink` strips only a link `link` could have produced —
  collision-free display text, exactly the target `link` would emit, target
  still present.

The additive, byte-for-byte reversible pair is the change-object model applied
to one kind of edit, and the untouched regions are the parser knowing what a
link, a fence, a frontmatter block, and a comment block are. See
[Change objects](../change-objects.md).

*Source: read from the aa_tool implementation — observation, not archie's
design.*

## Undecided

- Which frontmatter fields of a resource supply its surface strings.
- The link target when a resource is a folder rather than a file.
- Whether the dirty-tree guard, the dry run, the report, and `unlink` survive
  into archie.
- Output and exit codes: the corpus specifies neither for `archie link`.

## Related

- [Link resources](../../guides/link-resources.md) — the how-to.
- [Change objects](../change-objects.md) — the transactional model this pair
  follows.
- [Shapes, schemas, and resource types](../../concepts/shapes-schemas-resource-types.md)
  — where resource types live.
- [Command-line interface](index.md) — global options and the command table.
