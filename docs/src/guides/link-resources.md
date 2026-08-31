# Link resources

**Status:** Mixed — see markers.

`archie link` finds mentions of a resource in a corpus and turns them into links
to that resource. This guide describes it the way the design corpus does: by
describing an existing program that already does it, naming the one difference
archie introduces, and marking everything else as not yet decided.

## Before you start

**Decided.** archie's `link` is almost the behavior of the `link` subcommand of
Eduardo Burgos's `aa_tool` project. Asked on 2026-08-31 what `archie link folder/`
does, he answered: "It is ALMOST the same behavior as the `link` command in my
`aa_tool` project", and added, "That part is important to me."

**Decided.** The generalization is the difference. In his words, the same day: "In
archie it doesn't only works for contacts. It can work for whatever." `aa_tool`
links mentions of a person to that person's profile; archie links mentions of a
resource to that resource, whatever kind of resource it is.

**Unspecified.** What the "almost" is. Eduardo deferred the rest to a feature
discussion that has not happened — "We will discuss during the feature discussion
but I think you have the idea" — so every difference other than the widening from
people to any resource is unstated. The archie feature ledger records the entry
with the status `idea`, the earliest status in its vocabulary.

So read the steps below as a description of the reference implementation, with the
one stated generalization applied. Which of its properties archie inherits is what
the feature discussion has to settle. The
[archie link](../reference/cli/link.md) reference page carries the same
distinction rule by rule.

## Step 1: have the resources that supply the targets

In the reference implementation, link targets come from identity records: each
record's frontmatter carries a `name` and a list of `aliases`, keyed by a slug,
and the canonical record wins over a local one when both exist. Every name and
alias becomes a surface string that a mention can match, and the link target is a
relative path from the file being edited to that record.

**Unspecified for archie.** Which frontmatter fields of a resource provide its
surface strings, and how a target is chosen when the resource is a folder rather
than a file. Resource types themselves are remargin's — Eduardo settled that on
2026-08-30 — so what counts as a linkable resource is declared on remargin's side
and used on archie's. See
[Enforce resource types](enforce-resource-types.md).

## Step 2: run it over a folder

```console
$ archie link folder/
```

Eduardo's own command line, from 2026-08-30, and the only spelling of the command
in the corpus. No option of `archie link` is written down anywhere.

## Step 3: read what it produced

The reference implementation's behavior, read from its source. Each item is an
observation about that program.

- **Mentions become links.** Every collision-free mention of a surface string in
  the markdown files in scope is wrapped in a markdown link whose target is a
  relative path to the resource. The display text is the matched text, preserved
  exactly.
- **It is a pure function of the data and the content.** No heuristics and no
  scoring: the same records and the same files produce the same edits.
- **It matches longest first, on word boundaries.** The longest surface string
  that matches wins, and a match has to sit on Unicode word boundaries.
- **Collisions are refused, not guessed.** A surface string claimed by two or more
  resources is never linked, and every such string is reported as excluded.
- **It reports every edit.** The run emits a report in JavaScript Object Notation
  (JSON) listing each edit — the matched surface, the byte range in the original
  file, the file edited, the matched text, the slug of the resource linked to, and
  the relative path emitted as the target — together with the surfaces excluded
  for collision.

## What it never touches

The regions the run refuses to rewrite are as much a part of the behavior as the
rewriting is: frontmatter, fenced code blocks — remargin comment blocks among
them, because those are fenced blocks — inline code spans, existing markdown links
and wiki-links, and raw web addresses. A person is never linked inside their own
profile, and the documents that carry the alias data are never rewritten.

Knowing where those regions are is what archie's own parser is for. It is the same
discrimination a structural move needs when it rewrites inbound links — see
[Move and merge with link integrity](move-and-merge-with-links.md).

## Step 4: undo it

In the reference implementation, `unlink` is an exact inverse: it strips only a
link that `link` would have produced. The display text has to be a collision-free
surface, the target has to be exactly what `link` would emit, and the target
profile has to still exist — so a link to a missing target, being one `link` could
never have produced, is left alone.

**Unspecified.** Whether archie has an `unlink` at all. The corpus does not say.

## The guards the reference implementation has

`aa_tool` gives its `link` subcommand three options: one that allows it to run on
a git working tree with uncommitted changes, one that computes and reports the
edits without writing and exits with a non-zero status when edits are pending, and
one that writes the report to a named path. None of the three is written down for
archie, and neither is any equivalent.

## Why this command sits in archie at all

**Decided, as an observation recorded in the feature ledger.** The pair of
operations is the transactional change model applied to one kind of edit. The run
is additive — a link only wraps text that was already there, so stripping every
link it injected reproduces the file byte for byte — it is idempotent, because
text already inside a link is a region it skips, and its inverse is exact. Those
are the two promises a change object makes, in the small. See
[Change objects](../reference/change-objects.md).

## What is not decided

- Which frontmatter fields of a resource supply its surface strings.
- How a link target is chosen when the resource is a folder.
- Whether the guard against a dirty working tree and the preview run survive into
  archie.
- What an archie link report looks like.
- Whether archie has an inverse command.
- What the command returns to the shell. The exit-code table in the root design
  does not mention `archie link`, which is absent from that document entirely.

## Related pages

- [archie link](../reference/cli/link.md) — the reference page, with the
  implementation read rule by rule.
- [Change objects](../reference/change-objects.md) — the model this command
  follows in miniature.
- [Move and merge with link integrity](move-and-merge-with-links.md) — the other
  operation that rewrites references.
- [Enforce resource types](enforce-resource-types.md) — where the resources being
  linked to are declared.
