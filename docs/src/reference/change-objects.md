# Change objects

**Status:** Mixed — see markers.

A *change object* is archie's representation of one edit to a resource. This page
records what the design corpus states about change objects and what it does not.
The corpus states the model and says nothing about the artifact: no format, no
identity, no storage.

The sources quoted here carry a status of their own. The archie feature ledger
records this entry with the status `idea`, and the requirements document lists
it as a candidate goal that Eduardo Burgos has not yet accepted as a goal, and
the vision blueprint that puts the model on archie's side of the boundary
records that boundary as stated intent rather than as a design. So the model is
marked Proposed throughout, carrying the date he stated it.

## The model

**Proposed (stated by Eduardo on 2026-08-25).** Every change produces an object
that represents a change to a resource, and the model is the one git uses. The
feature ledger records it in his words, under "Transactional changes": "I want
changes to be transactional." Given a base resource and a chain of changes
applied in sequence, you obtain a byte-identical latest version; given the
latest version and the same changes unapplied in reverse, you eventually get
back to the original.

Three properties follow from that sentence and are stated in the corpus rather
than derived here:

- **A base plus a chain.** A change is not a snapshot. The latest version is what
  the base becomes after the chain has been applied to it.
- **Byte-identical replay.** Applying the chain to the base reproduces the latest
  version exactly, down to the bytes.
- **Reversibility.** Unapplying the chain in reverse returns the original.

**Proposed (stated by Eduardo on 2026-08-25).** The reason the model can hold is
stated with it: the content is linted markdown with a known shape. The change
model therefore depends on the linter and on the shape language — see
[Shape language](shape-language.md) .

**Proposed (stated by Eduardo on 2026-08-25).** Every write through remargin is
such a change — in his words, "ANY CHANGE". The remargin feature ledger records
the same statement from remargin's side, in its entry "Every write is a
transaction": remargin's entire write path is where the model applies, and no
write bypasses it. The change model itself is archie's; the write path is
remargin's. The program's vision blueprint places the transactional change model
on archie's side when it records the boundary he drew that day, and says in the
same breath that the boundary is stated intent and not yet a design.

## What the model generalizes

**Proposed (stated by Eduardo on 2026-08-25).** The change model generalizes the
integral structural operations captured on 2026-08-16, which were the first
place a transaction appeared in the corpus. That capture records what happened
when a folder of 216 files was moved inside the vault: the move itself preserved
the comment threads that live inside the files, and everything else — finding
every inbound reference, rewriting it, deciding that document bodies get
rewritten while comments stay frozen as historical record, and verifying the
result — was an agent improvising around a tool that knows nothing about links.

The capability that capture asks for: a structural operation is a transaction.
Move the folder, rewrite every inbound link, revalidate the indexes, atomically or
not at all. It contrasts that with the rules in the root design, which detect a
broken reference after the fact.

**Decided.** That the structural operations themselves are archie's territory is
settled separately from the change model that generalizes them. Eduardo
confirmed it, and the vision blueprint records the confirmation as its one piece
of concrete evidence for what "archie becomes the core of remargin" means
mechanically.

**Proposed, in every detail.** The same capture says the structural semantics
get designed later, and names what is missing: which spellings of a reference
get rewritten — root-anchored paths, relative paths, wiki-links, embeds — and
which are preserved, such as historical spellings, quoted commands, and signed
comments; how the operation reports what it rewrote; and what a dry run does. It
also describes a merge-move that unions per entry into an existing destination
under an explicit collision policy of fail, skip, or overwrite, defaulting to
fail, removes the source folder once it is emptied, and moves nothing at all if
any entry is refused. See [Structural operations](cli/structural-operations.md)
.

## The one worked example in the corpus

**Proposed, as an observation recorded in the corpus.** The archie feature
ledger, in its entry on `link`, reads the `link` and `unlink` pair of Eduardo's
`aa_tool` project as the change-object model applied to one kind of edit. The
ledger holds that entry with the status `idea`, so the reading is as unsettled
as the model it reads into. That implementation is additive — a link only wraps
text that was already there, so stripping every link it injected reproduces the
file byte for byte — it is idempotent, and `unlink` is its exact inverse,
stripping only a link that `link` would have produced. The
additive-and-exactly-reversible pair is what a change object has to be, in the
small.

archie's `link` generalizes that behavior; Eduardo said on 2026-08-31 that in
archie it is not restricted to contacts and "can work for whatever", with the rest
deferred to a feature discussion. See [archie link](cli/link.md).

## Where versions live

**Proposed (stated by Eduardo on 2026-08-25).** Version tracking sits above a
pluggable storage backend. His words, in the feature ledger entry
"Backend-agnostic version tracking": "I also need to be able to track versions
in a so abstracted way that no one should even worry about whether the backend
is a filesystem, git, AWS S3, Minio, MongoDB or any other." He accepted the
trade-off in the same breath: "doing this will be easier for some backends than
for others, so we can maybe have 'not implemented' some features for some
backends, and that could be ok."

The corpus does not connect the two halves. It does not say whether a change
object is what a backend stores, whether a version is a change object or a
materialized document, or which of the two the backend abstraction is defined
over. See [Backends and capabilities](../concepts/backends-and-capabilities.md).

## What the corpus does not specify

Beyond the model itself not being accepted as a goal, five things about the
artifact are absent.

- **The format.** No document says what a change object contains, how it is
  serialized, or whether it records the edit as a patch, as an operation on the
  document tree, or as something else.
- **The identity.** No document says how one change object is named or referred
  to, how a chain is ordered, or how a base version is identified.
- **The storage.** No document says where change objects are kept, how long they
  are kept, or whether the chain lives beside the document, inside it, or in the
  backend.
- **The transaction boundary for a query-driven change.** Whether one
  query-driven mass modification is a single change object covering every document
  touched, or one change object per document, is an open item on the grammar
  document. See [Query grammar](query-grammar.md).
- **What happens when replay fails.** The model states that replay is
  byte-identical; nothing says what archie does when it is not — when a document
  has been edited outside archie, for instance.

> **Suggestion (documentation author):** the model is fixed and the artifact is
> undescribed, which leaves a reference page unable to say what a reader would
> hold in their hands. Three decisions would close that gap, and all three are
> Eduardo's. **Format:** whether a change is recorded as a textual patch over the
> document, as an operation over the parsed document tree — an insertion at an
> addressable path, for example — or as both, one derived from the other. An
> operation over the tree is the form that matches the rest of the design, since
> the tree is what the grammar addresses and what a shape governs, but nothing in
> the corpus says so. **Identity:** whether a change object is named by a content
> hash in the manner of git, by a sequence number within one document's chain, or
> by an identifier assigned by the backend — and whether a chain covering several
> documents is itself a named object. **Storage:** whether the chain is the
> backend's responsibility, so that the filesystem backend keeps it in a sidecar
> file and the git backend keeps it as commits, or whether archie defines one
> storage layout every backend implements. Nothing here has standing until he
> decides it.

## Where this leads

- [Changes and versions](../concepts/changes-and-versions.md) — the concept page.
- [Structural operations](cli/structural-operations.md) — moves and merges as
  transactions.
- [Query grammar](query-grammar.md) — mutation, and where a transaction ends.
- [Backends and capabilities](../concepts/backends-and-capabilities.md) — where a
  version is kept.
