# Backends and capabilities

**Status:** Mixed — see markers.

A version of a document has to be kept somewhere. archie's design puts that
"somewhere" behind an abstraction, so that the engine tracks versions without the
caller knowing whether the bytes end up in a folder on a disk, in a git
repository, or in an object store on someone else's servers. This page records
what the design corpus says about that abstraction, and it is short, because the
corpus says one paragraph's worth.

## The one statement in the corpus

**Proposed.** In Eduardo's words, on 2026-08-25: "I also need to be able to track
versions in a so abstracted way that no one should even worry about whether the
backend is a filesystem, git, AWS S3, Minio, MongoDB or any other." The archie
feature ledger records it with the status `idea`, the requirements document holds
it among the candidate goals awaiting acceptance, and the vision blueprint's
record of the 2026-08-25 boundary places backend-agnostic version tracking in
archie rather than in remargin.

Five backends are named as examples, and the sentence ends with "or any other", so
the five are not a list of what is supported. A filesystem is a plain directory
tree; git is a version-control system that already keeps its own history; Amazon
Web Services' S3 and Minio are object stores, and Minio offers the same interface
as S3 on hardware one runs oneself; MongoDB is a document database.

**Proposed.** Uneven support is accepted in advance, and that exemption is what
the word "capabilities" in this page's title refers to. In his words: "doing this
will be easier for some backends than for others, so we can maybe have 'not
implemented' some features for some backends, and that could be ok." So the
abstraction is not required to reduce every backend to the same set of operations.
A backend may decline to implement part of the interface and remain a legitimate
backend.

## Why the abstraction has to come after the document abstraction

The corpus records the dependency between two of the 2026-08-25 ideas. Abstracting
the document and the schema away from files is named as the prerequisite for both
the change model and the backend abstraction: version tracking over a pluggable
backend only means something once a document can exist without a file underneath
it. See [Documents and schemas](documents-and-schemas.md) for that abstraction and
[Changes and versions](changes-and-versions.md) for the model of what gets
tracked.

One thing worth stating plainly, because git appears in the corpus twice in
different roles: git is named as one candidate storage backend among five, and
separately as the model the change chain is patterned on — a base plus a sequence
of changes that replays to a byte-identical latest version. The two mentions are
independent. Nothing in the corpus says that the change model requires git as a
backend, and nothing says that using git as a backend would delegate the change
chain to git's own history.

## What the corpus does not say

The absences here are large, and this book fills none of them.

- **The interface.** Which operations a backend provides is written down nowhere,
  under any name.
- **What is stored.** Whether a backend stores documents, change objects, or both.
- **How "not implemented" is surfaced.** Whether a caller learns that a backend
  cannot do something by asking beforehand or by attempting the operation and
  receiving an error.
- **How a backend is chosen or configured.** The root design's configuration file
  has no backend section, and predates this direction entirely.
- **Which operations are indispensable.** Nothing says whether any part of the
  interface is required of every backend, so nothing marks where the exemption
  stops.

> **Suggestion (documentation author):** this page would be more useful with a
> capability matrix — backends as rows, engine operations as columns, each cell
> reading supported or not implemented — because "not implemented is acceptable"
> is only actionable if a reader can tell which combinations are which. The corpus
> lists backends and grants the exemption but contains no such matrix, so this
> book publishes none. Two decisions from Eduardo would be needed before one could
> be written. First, the list of operations that form the columns, which follows
> from the version-tracking interface that is not yet designed. Second, whether a
> backend declares its capabilities up front, so that a caller can consult the
> matrix at run time and the matrix is a real object in the system, or whether
> "not implemented" is only ever an error returned from an attempted call, in
> which case the matrix is documentation and nothing more.

## What this means for the guide

The [Choose a backend](../guides/choose-a-backend.md) guide can only go as far as
the corpus does. A reader looking for advice on which of the five named backends
to use will not find it there, because the design contains no basis for such
advice — no interface, no comparison, and no worked example of any backend.
