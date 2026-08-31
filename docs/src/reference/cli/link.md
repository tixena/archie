# archie link

**Status:** Mixed — see markers.

`archie link` turns mentions of a resource into links to that resource. Eduardo
Burgos described it by pointing at an existing program of his own rather than by
specifying it, so this page states what that program does, states the one
difference he has named, and marks everything else as not yet decided.

## Synopsis

```console
$ archie link folder/
```

Eduardo's own command line, written on 2026-08-30 on the planning document. It is
the only spelling of `archie link` anywhere in the corpus. No option of
`archie link` is written down.

## What is decided

**Decided.** `archie link` is almost the behavior of the `link` subcommand of
Eduardo's `aa_tool` project. When asked on 2026-08-31 what `archie link folder/`
does, he answered: "It is ALMOST the same behavior as the `link` command in my
`aa_tool` project", and added, "That part is important to me." The
implementation he pointed at is `~/src/tixena/aa_tool/src/link.rs`.

**Decided.** In archie it is not restricted to people. Eduardo, 2026-08-31: "In
archie it doesn't only works for contacts. It can work for whatever." That is the
generalization: `aa_tool` links mentions of a person to that person's profile;
archie links mentions of a resource to that resource, whatever kind of resource
it is.

**Unspecified.** What the "almost" is. Eduardo said the rest waits for a feature
discussion that has not happened: "We will discuss during the feature discussion
but I think you have the idea." So every difference between `aa_tool`'s behavior
and archie's, other than the widening from people to any resource, is unstated.

The feature ledger records `link` with the status `idea`, which is the earliest
status in that ledger's vocabulary and means it has not been accepted as
something archie will do.

## What the reference implementation does

Read from `~/src/tixena/aa_tool/src/link.rs`. This is an observation about an
existing program, not a statement about archie. Which of these properties archie
inherits is what the feature discussion has to settle.

- **It wraps mentions in links.** Every collision-free mention of a person's name
  or alias found in the markdown files in scope is wrapped in a markdown link
  whose target is a relative path to that person's profile. The display text is
  the matched text, preserved exactly.
- **It is a pure function of the data and the content.** No heuristics and no
  scoring. Link targets come from identity records: the `name` field of a
  record's frontmatter plus its `aliases`, keyed by a slug.
- **It is additive.** A link only wraps text that was already there, so stripping
  every injected link reproduces the file byte for byte.
- **It is idempotent.** Text already inside a link is a region the run skips, so a
  second run changes nothing.
- **It is collision-guarded.** A surface string claimed by two or more people is
  never linked, and every such string is reported as excluded.
- **It leaves whole regions alone.** Frontmatter, fenced code blocks — remargin
  comment blocks among them, because those are fenced blocks — inline code spans,
  existing markdown links and wikilinks, and raw web addresses are never
  rewritten. A person is never linked inside their own profile, and documents
  that carry alias data are never rewritten.
- **It matches longest first, on word boundaries.** The longest surface string
  that matches wins, and a match has to sit on Unicode word boundaries.
- **It reports what it did.** The run emits a report in JavaScript Object Notation
  (JSON) listing every edit — the matched surface, the byte range in the original
  file, the file edited, the matched text, the slug of the person linked to, and
  the relative path emitted as the target — plus the collision-excluded surfaces.
- **It has a guard and a dry run.** It refuses to run on a git working tree with
  uncommitted changes unless told to allow it, and its dry run computes and
  reports the edits without writing, exiting with a non-zero status when edits are
  pending.
- **It has an exact inverse.** The `unlink` subcommand strips only a link that
  `link` would have produced: the display text has to be a collision-free surface,
  the target has to be exactly what `link` would emit, and the target profile has
  to still exist. A link to a missing target is one `link` could never have
  produced, so `unlink` leaves it alone.

The options `aa_tool` gives its `link` subcommand are `--allow-dirty`,
`--dry-run`, and `--report <PATH>`. None of them is written down for archie.

## What generalizing means

**Decided, in outline.** Widening from people to any resource changes what
supplies the link targets. In `aa_tool` the targets come from person records with
a `name` and `aliases`; in archie the targets would come from resources of any
kind. Resource types are remargin's, not archie's — Eduardo stated that on
2026-08-30, and it is recorded as a non-goal in the requirements document — so
what counts as a resource, and which of its fields supply the surface strings to
match, is remargin's to declare and archie's to enforce and use.

**Unspecified.** Everything below the outline: which frontmatter fields of a
resource provide its surface strings, how a resource's link target is chosen when
a resource is a folder rather than a file, whether the git-cleanliness guard and
the dry run survive into archie, what an archie link report looks like, and
whether archie has an `unlink`.

An observation recorded in the feature ledger, and repeated here because it
explains why this command sits in archie at all: the additive, byte-for-byte
reversible pair of `link` and `unlink` is the change-object model applied to one
kind of edit, and the regions the run refuses to touch are the parser knowing
what a link, a fence, a frontmatter block, and a comment block are.

## Examples

```console
$ archie link folder/
```

Eduardo's own example. The design specifies no output for it, so none is shown.

## Exit codes

The root design document's exit-code table does not mention `archie link`; the
command is absent from that document entirely. The reference implementation exits
with a non-zero status when a dry run finds pending edits, which is a property of
`aa_tool`, not a statement about archie. See [Exit codes](../exit-codes.md).

## Related pages

- [Link resources](../../guides/link-resources.md) — the task-shaped guide to
  this command.
- [Change objects](../change-objects.md) — the transactional model whose shape
  `link` and its inverse follow.
- [Shapes, schemas, and resource types](../../concepts/shapes-schemas-resource-types.md)
  — where resource types live and who declares them.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
