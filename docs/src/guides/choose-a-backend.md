# Choose a backend

**Status:** Proposed — see markers.

A *backend* is where the versions of a document are kept. archie's design puts
that somewhere behind an abstraction, so that the engine tracks versions without
the caller knowing whether the bytes end up in a folder on a disk, in a git
repository, or in an object store.

This guide is short, and the reason is worth stating at the top: there is nothing
to choose yet. The design corpus contains one paragraph on backends. It names five
of them as examples, it grants an exemption for uneven support, and it defines no
interface, no configuration, and no way to select one. A reader looking for advice
on which backend to use will not find it here, because the design contains no
basis for such advice.

## What the corpus says

**Proposed.** In Eduardo Burgos's words, on 2026-08-25: "I also need to be able to
track versions in a so abstracted way that no one should even worry about whether
the backend is a filesystem, git, AWS S3, Minio, MongoDB or any other." The archie
feature ledger records it with the status `idea`, the requirements document holds
it among the candidate goals he has not yet accepted as goals, and the program's
vision blueprint places backend-agnostic version tracking on archie's side of the
boundary he drew that day.

The sentence ends with "or any other", so the five names are examples rather than
a supported list.

| Backend named | What it is |
|---|---|
| A filesystem | A plain directory tree. |
| git | A version-control system that already keeps a history of its own. |
| Amazon Web Services' S3 | An object store run as a service. |
| Minio | An object store offering the same interface as S3 on hardware you run yourself. |
| MongoDB | A document database. |

## The "not implemented" contract

**Proposed.** Uneven support is accepted in advance, in the same breath as the
abstraction. In his words: "doing this will be easier for some backends than for
others, so we can maybe have 'not implemented' some features for some backends,
and that could be ok."

Read carefully, that is a real design constraint rather than a hedge. The
abstraction is not required to reduce every backend to the same set of operations.
A backend may decline to implement part of the interface and still be a legitimate
backend, which means a caller cannot assume that an operation available on one
backend is available on the next.

**Not in the corpus.** How a caller learns that a backend cannot do something —
by asking beforehand, or by attempting the operation and receiving an error. The
difference decides whether a program can plan around a limitation or only react to
one, and nothing in the design settles it.

## What a backend holds

**Not in the corpus.** Whether a backend stores documents, change objects, or
both. The two halves of the design are not connected: the change model says a
document's history is a base plus an ordered chain of changes that replays to a
byte-identical latest version, and the backend statement says versions are tracked
above a pluggable store, and no document says which of the two the store is
defined over. See [Change objects](../reference/change-objects.md).

**Decided, as a dependency.** The backend abstraction comes after the document
abstraction. Version tracking over a pluggable store only means something once a
document can exist without a file underneath it, and abstracting the document and
the schema away from files is recorded as the prerequisite for it. See
[Documents and schemas](../concepts/documents-and-schemas.md).

## One thing not to confuse

git appears twice in the corpus, in unrelated roles. It is named as one candidate
store among five, and separately as the model the change chain is patterned on — a
base plus a sequence of changes that replays exactly. Nothing says the change model
requires git as a store, and nothing says that using git as a store would delegate
the chain to git's own history.

## What you cannot do today

- **Select a backend.** No configuration key names one. The root design's
  configuration file has no backend section at all and predates this direction.
- **Ask what a backend supports.** No capability list, no matrix, no query. The
  [Backends and capabilities](../concepts/backends-and-capabilities.md) concept
  page explains why this book publishes no capability matrix and what would have
  to be decided before one could exist.
- **Write against the interface.** No operation of a backend is named anywhere,
  under any spelling.

## Related pages

- [Backends and capabilities](../concepts/backends-and-capabilities.md) — the
  concept page, with the full list of what the corpus leaves out.
- [Changes and versions](../concepts/changes-and-versions.md) — what would be
  tracked.
- [Change objects](../reference/change-objects.md) — the model, and its
  unconnected relationship to storage.
- [Documents and schemas](../concepts/documents-and-schemas.md) — the abstraction
  this one depends on.
