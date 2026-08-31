# Structural operations

**Status:** Mixed — see markers.

Moving a folder, renaming a document, archiving something, and merging one folder
into another are changes to the shape of a corpus rather than to the contents of
one document. The design corpus records that they belong to archie and that they
have to be transactions. It records no command spelling for any of them, which is
why this book documents them on one page rather than one page per verb.

## What is decided

**Decided.** Structural operations are archie's territory. The vision blueprint
records this as part of the evidence for what "archie becomes the core of
remargin" means: structural operations on a corpus — move, rename, archive — are
integral transactions that keep links valid by construction, rather than
validating and repairing afterwards.

**Decided.** The reason, from the capture that recorded it on 2026-08-16. Moving
one folder of 216 files into a new home was one conceptual operation that fell
apart into several: a tracked move that preserved the comment threads living
inside the files, then a hand-run search of the whole corpus for inbound
references, then a search-and-replace with hand-made carve-outs so that historical
spellings were left alone, then a judgment about which references to rewrite —
document bodies yes, comment threads no, because those are a historical record —
and then a verification pass. All of that intelligence lived in the person doing
it, not in the tool.

**Decided, as the shape of the answer.** archie owns the corpus as a structured
graph, with typed resources and links as first-class edges, so a structural
operation moves the folder, rewrites every inbound link, revalidates the
inheritance and the index files, and does all of it or none of it. The capture
contrasts this with the validation rules that detect broken references after the
fact — the rule for assets nothing references, and the rule that a relation's
target must exist — and calls it a third class of operation: one that cannot break
integrity in the first place. The comparison it draws is to an editor's rename of
a symbol, as against a checker that flags dangling references afterwards.

## What is proposed

**Proposed.** Every detail. The capture states plainly that the semantics get
designed later, and lists three things that are undesigned:

- **Which reference spellings get rewritten, and which are preserved.** Candidates
  for rewriting: paths anchored at the corpus root, relative paths, wiki-links, and
  embeds. Candidates for preserving: historical spellings, commands quoted inside
  a document, and signed comment blocks whose content must not change.
- **How the operation reports what it rewrote.**
- **What a dry run does.**

**Proposed.** The merge-move. The same capture describes a second operation, drawn
from the same reorganization: merging a folder of 99 files into a destination that
already existed and already had contents. The move primitive available at the time
could not express it — pointed at a non-empty destination it only proceeded when
forced, and forcing deleted the destination's contents first, which would have
destroyed what was already there. The workaround was nineteen child-by-child
moves, which left an empty source folder behind that then had to be removed by
hand.

The capture names three defects in one operation — no merge mode; a force mode
whose meaning is destructive replacement rather than union; and no cleanup of a
source folder a move has emptied — and states the capability that answers them: a
merge-move that unions entry by entry into an existing destination, under a
collision policy chosen per invocation from failing, skipping, and overwriting,
defaulting to failing; that removes the source folder once it is emptied; that
rewrites every moved file's inbound links, because it composes with the operation
above; and that is atomic, so that if any entry is refused, nothing moves at all.

## Command spellings

**Unspecified.** No document in the corpus gives a command name for any of these
operations. They are absent from the root design document's command list and
absent from Eduardo Burgos's list of 2026-08-30.

> **Suggestion (documentation author):** the operations need spellings before they
> can be documented as commands, and Eduardo would have to choose them. Three
> shapes are available and the corpus prefers none: one verb per operation
> (`archie mv`, `archie rename`, `archie merge`, `archie archive`), which matches
> the way `ls`, `get`, and `rm` were named in his own list; a single verb with a
> mode (`archie move --merge`), which keeps the collision policy and the dry run in
> one place; or no command at all, on the ground that these are engine operations
> remargin calls and the command line does not need to expose them. The third is
> live: the vision blueprint asks whether the engine should be designed around
> remargin's interface with the command line as a thin shell over it. Every verb
> written in this paragraph is this book's invention and appears nowhere in the
> design.

Because no spelling exists, this page shows no command line. A console block here
would be an invention rather than a quotation.

## What connects this to the rest of archie

**Proposed (stated by Eduardo on 2026-08-25).** The transactional change model
generalizes these operations to every change. He stated that day that he wants
changes to be transactional: any change through remargin produces an object
representing a change to a resource, modeled the way git operates, so that a
base resource plus a chain of changes applied in order yields a byte-identical
latest version, and unapplying the chain in reverse returns the original. His
feature ledger records that this generalizes the integral structural operations
captured here — move and merge as transactions — to every kind of change.

**Decided, in outline.** Rewriting a reference without touching a signed comment
block or a fenced code block is the same discrimination `archie link` makes when
it refuses to rewrite frontmatter, fenced blocks, inline code, existing links, and
raw web addresses. Both rest on archie's own parser knowing what those regions
are.

## Exit codes

The root design document's exit-code table does not cover these operations, which
that document does not contain. See [Exit codes](../exit-codes.md).

## Related pages

- [Move and merge with link integrity](../../guides/move-and-merge-with-links.md)
  — the task-shaped guide.
- [Change objects](../change-objects.md) — the transactional model these
  operations generalize into.
- [archie link](link.md) — the command that rewrites document text without
  touching protected regions.
- [archie rm](rm.md) — removal, which the structural-operations capture does not
  cover.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
