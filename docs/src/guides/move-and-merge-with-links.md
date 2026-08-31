# Move and merge with link integrity

**Status:** Mixed — see markers.

Moving a folder, renaming a document, archiving something, and merging one folder
into another change the shape of a corpus rather than the contents of one
document. archie's design treats them as one class of operation with one
requirement: every link that pointed into what moved still points at it
afterwards, because the operation rewrote those links as part of itself.

This guide has no command lines in it. No document in the design corpus gives a
command name to any of these operations, so a console block here would be an
invention rather than a quotation. What the guide can do is tell you what the
design promises, what it does not promise yet, and what that means for a
reorganization you have to do today.

## What the design promises

**Decided.** These operations are archie's. The program's vision blueprint records
Eduardo Burgos confirming it, and treats it as the one piece of concrete evidence
it holds for what "archie becomes the core of remargin" means mechanically:
structural operations on a corpus — move, rename, archive — are integral
transactions that keep links valid by construction, rather than validating and
repairing afterwards.

**Decided, as the shape of the answer.** archie owns the corpus as a structured
graph, with typed resources and links as first-class edges. So one operation moves
the folder, rewrites every inbound link, revalidates the inheritance and the
folder index files, and does all of that or none of it. The capture that recorded
the capability contrasts it with the rules that detect a broken reference after
the fact — the rule for assets nothing references, and the rule that a relation's
target must exist — and calls it a third class of operation, one that cannot break
integrity in the first place. The comparison it draws is to an editor's rename of
a symbol, against a checker that flags dangling references afterwards.

## Where the requirement came from

**Decided.** A real reorganization on 2026-08-16 is the evidence. Moving one
folder of 216 files into a new home was a single conceptual operation, and it fell
apart into five:

1. A tracked move, which preserved the comment threads because those live inside
   the files that moved.
2. A hand-run search of the whole corpus for inbound references.
3. A search-and-replace with hand-made carve-outs, because references written with
   an older path prefix had to be excluded and the pattern engine in use could not
   express the exclusion cleanly.
4. A judgment about which references to rewrite: document bodies yes, comment
   threads no, because those are a historical record.
5. A verification pass.

All of that intelligence lived in the person doing the work rather than in the
tool. That is the gap the operation is meant to close.

## The merge-move

**Proposed.** A second capability came out of the same day. A folder of 99 files
had to merge into a destination that already existed and already had contents. The
move primitive available could not express it: pointed at a non-empty destination
it proceeded only when forced, and forcing deleted the destination's contents
first, which would have destroyed what was already there. The workaround was
nineteen child-by-child moves, which left an empty source folder behind that then
had to be removed by hand.

The capture names three defects in one operation — no merge mode; a force mode
whose meaning is destructive replacement rather than union; and no cleanup of a
source folder a move has emptied — and states the capability that answers them. A
merge-move that:

- unions entry by entry into an existing destination;
- takes a collision policy chosen per invocation from failing, skipping, and
  overwriting, defaulting to failing;
- removes the source folder once it has been emptied;
- rewrites every moved file's inbound links, because it composes with the move
  above;
- is atomic, so that if any single entry is refused, nothing moves at all.

## What is still undesigned

**Proposed, in every detail.** The capture states plainly that the semantics get
designed later, and names three things nobody has settled.

- **Which reference spellings get rewritten, and which are preserved.** Candidates
  for rewriting: paths anchored at the corpus root, relative paths, wiki-links,
  and embeds. Candidates for preserving: historical spellings, commands quoted
  inside a document, and signed comment blocks whose content must not change.
- **How the operation reports what it rewrote.**
- **What a dry run does.**

**Unspecified.** The command spelling. These operations are absent from the root
design document's command list and absent from the list Eduardo wrote on
2026-08-30. The
[Structural operations](../reference/cli/structural-operations.md) reference page
sets out the three shapes a spelling could take and marks the choice as Eduardo's;
this guide adds nothing to it and invents no verb.

## What you can do today

Nothing through archie: it has no binary, and these operations have no names. Two
things are worth knowing while that is true.

The validation rules do not substitute for the operation. The rule for assets
nothing references and the rule that a relation's target must exist both report a
break after it has happened, which is the arrangement the capture argues against.
See [Validation rules](../reference/validation-rules.md).

And `archie link` is the one command in the corpus that already rewrites document
text without disturbing the regions it must not touch — frontmatter, fenced
blocks, inline code, existing links, and raw web addresses. The discrimination a
link rewrite needs during a move is the same one, resting on archie's own parser
knowing what those regions are. See [Link resources](link-resources.md).

## How this fits the rest of archie

**Proposed (stated by Eduardo on 2026-08-25).** The transactional change model
generalizes these two operations to every change. He stated that any change
through remargin produces an object representing a change to a resource, modeled
the way git operates, so that a base plus a chain of changes replays to a
byte-identical latest version and unapplies in reverse to the original — and his
feature ledger records that model as the generalization of the move and merge
captured here. See [Change objects](../reference/change-objects.md) .

## Related pages

- [Structural operations](../reference/cli/structural-operations.md) — the
  reference page, including the question of what these operations should be
  called.
- [Changes and versions](../concepts/changes-and-versions.md) — the reorganization
  in full and the model it produced.
- [Change objects](../reference/change-objects.md) — replay, unapply, and what the
  corpus does not say about the artifact.
- [Link resources](link-resources.md) — the one rewrite archie's design describes
  in detail.
- [archie rm](../reference/cli/rm.md) — removal, which this class of operation
  does not cover.
