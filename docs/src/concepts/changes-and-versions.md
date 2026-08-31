# Changes and versions

**Status:** Mixed — see markers.

archie's model of editing is not "open a file, write it back". Every change is an
object: a value that describes what was done to a resource, that can be applied to
produce the next version of that resource, and that can be undone to produce the
previous one. A document's history is the base document plus the ordered chain of
those objects. This page records what the design corpus says about that model,
where it came from, and what it does not yet say.

## Every change is an object

**Proposed.** In Eduardo's words, on 2026-08-25: "I want changes to be
transactional." Every write through remargin — "ANY CHANGE", in his emphasis —
produces an object that represents a change to a resource. He named the model
directly: it works the way git works. Given a base resource and a chain of changes
applied one after another, you obtain a byte-identical latest version. Given the
latest version and the same changes unapplied in reverse order, you eventually
arrive back at the original.

**Proposed.** He also gave the reason the guarantee is reachable at all: the
content is linted markdown with a known shape. A change to arbitrary text can only
be described as a text difference; a change to a document whose structure is known
can be described in terms of that structure.

The archie feature ledger holds this with the status `idea` and the requirements
document holds it among the candidate goals awaiting acceptance, which is why it
is marked Proposed. Where it belongs is settled: the vision blueprint's record of
the 2026-08-25 boundary places the transactional change model in archie.

**Decided.** The write path it applies to is remargin's, and nothing bypasses it.
The remargin feature ledger records the division: the change model itself is an
archie engine capability, and remargin's entire write path is where it applies.

## Two claims, and what each one requires

The model makes two separate promises, and it is worth keeping them apart because
they fail differently.

*Replay to a byte-identical latest version.* Applying the chain to the base has to
reproduce the current document exactly, byte for byte — not a document that means
the same thing, or renders the same way. That is what makes the chain a faithful
record rather than a summary of what happened.

*Unapply back to the original.* Reversing the chain has to arrive at the document
as it was. Every change object therefore has to carry enough to undo itself, not
only enough to redo itself.

The corpus states both as the goal. It does not describe how a change object is
represented, what its parts are, or how those two guarantees are checked. The
[Change objects](../reference/change-objects.md) reference page records the same
absence.

## Where the model came from: structural operations

**Decided.** Structural operations on a corpus — move, rename, merge, archive — as
integral transactions that keep links valid by construction are archie's
territory. The vision blueprint records Eduardo confirming it, and treats it as
the one piece of concrete evidence toward what "archie becomes the core of
remargin" means mechanically. The archie feature ledger presents the general
change model as the generalization of these two specific operations to every kind
of change.

The capture that recorded them describes a reorganization on 2026-08-16 in which
both were needed and neither existed.

### Integral moves

**Decided.** That integral moves belong to archie is settled; what happened on
2026-08-16 is the evidence behind it. Moving a folder of 216 files into a new home
was one conceptual operation. What it actually decomposed into: a tracked move,
which preserved the comment threads because those live inside the files that
moved; then
a manual vault-wide hunt for inbound references; then a regular-expression replace
with hand-crafted carve-outs, because historical references written with an older
path prefix had to be excluded and the regular-expression engine had no lookbehind
to exclude them cleanly; then a judgment call that document bodies get rewritten
while comments stay frozen as a historical record; then a verification pass. All
of that intelligence lived in an agent improvising around the tool.

The capability the capture draws from it: archie owns the corpus as a structured
graph, with typed resources and links as first-class edges, so a move rewrites
every inbound link, revalidates inheritance and folder indexes, and either
completes or does nothing. The corpus states the distinction as a third class of
verb. Rules that check and then repair — the orphaned-asset rule, the rule that
relation targets must exist — detect a broken reference after the fact. An
integral move cannot break the reference in the first place. The analogy the
capture uses is an editor's rename-symbol command against a linter that flags
dangling references afterwards.

**Proposed, in every detail.** The capture is explicit that the semantics get
designed later: which reference spellings get rewritten — root-anchored paths,
relative paths, wiki-links, embeds — and which are preserved as historical
spellings, quoted commands, or signed comments; how the operation reports what it
rewrote; and what a dry run does.

### Merge-move

**Proposed.** The second capability came from the same day. A folder of 99 files
had to merge into a destination folder that already existed and already had
contents. The move primitive could not express it: pointed at a non-empty
destination it proceeds only with a force option, and force deletes the
destination subtree before moving, which would have destroyed what was already
there. The workaround was nineteen child-by-child moves, six of directories and
thirteen of files, which left an empty source directory behind that then could not
be removed without a manual command.

The capture names three defects in one operation: no merge mode at all; a force
mode whose meaning is destructive replacement rather than union; and no source
cleanup when a move empties the source.

The capability it proposes: a merge-move that unions entry by entry into an
existing destination, under a collision policy chosen per invocation — fail, skip,
or overwrite, defaulting to fail — that removes the source directory once it is
emptied, that rewrites the inbound links of every moved file by composition with
the integral move above, and that is atomic, so that if any single entry is
refused nothing moves at all.

**Not in the corpus.** No command spelling exists for either operation. Neither
appears in the list of commands Eduardo wrote on 2026-08-30, and the root design
predates both. The
[Structural operations](../reference/cli/structural-operations.md) reference page
documents the capabilities rather than a verb.

## A worked instance: additive, reversible linking

**Decided.** One operation in this family already exists in working code, in
Eduardo's `aa_tool` project, and the archie feature ledger records the reading it
supports. That tool's `link` command wraps every collision-free mention of a
person's name or alias in a relative-path link to that person's profile. It states
two invariants that are exactly the two promises of the change model, applied to
one kind of edit: it is additive, so stripping every link it injected reproduces
the file byte for byte; and its `unlink` command is an exact inverse, stripping
only a link that `link` itself would have produced.

The same source shows what a parser has to know for such an edit to be safe. It
never rewrites frontmatter, fenced code blocks — remargin comment blocks
included — inline code spans, existing links and wiki-links, or raw URLs. The
ledger names that plainly: the skip zones are the parser knowing what a link, a
fence, a frontmatter block, and a comment block are.

**Decided.** archie's own `link` generalizes it. Eduardo, 2026-08-31: "In archie
it doesn't only works for contacts. It can work for whatever" — any resource, not
only people. The rest is deferred to a feature discussion that has not happened.

## What is not settled

**Open in the corpus.** Where a transaction ends. The grammar document lists it
among the unanswered questions: when one query drives a modification across many
documents, is that one change object covering every document touched, or one
change object per document? Eduardo's own example of a mass modification —
appending a section carrying the current date to every element a query returns —
is exactly the case that needs the answer.

**Open in the corpus.** Which mutation verbs exist at all. Appending is the only
verb Eduardo has written down, in the command `archie write` with its `--append`
option and in the mass-modification example. Whether the language also carries
setting, removing, replacing, and moving is listed as unspecified.

**Not in the corpus.** How a change object relates to storage. Version tracking is
abstracted over pluggable backends — see
[Backends and capabilities](backends-and-capabilities.md) — but nothing says
whether a backend stores documents, change objects, or both, or what a backend has
to provide for replay and unapply to work.
