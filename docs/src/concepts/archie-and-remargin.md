# archie and remargin

**Status:** Mixed — see markers.

remargin is Eduardo Burgos's document-collaboration system and his main working
tool, used every day alongside Obsidian: markdown documents that carry threaded
comments, participant identities and signatures, permission realms, a Model
Context Protocol (MCP) server, a command line, and an Obsidian plugin. archie is
the engine being designed underneath it.

This page records what each side holds, what archie contributes to the parts of
remargin that are still being designed, and which parts of the line between the
two the design corpus has not drawn yet.

## archie is being built to become remargin's core

**Decided.** archie will become the core of remargin, and that is the reason
archie is being built. Eduardo stated it on 2026-08-16 and it is recorded as
decision D1 in the vision blueprint of the Remargin Vision program. The blueprint
draws one consequence from it immediately: archie's primary consumer is remargin
rather than the note-taking experience of any one vault, so archie's engine
layers are designed as a library before they are designed as a command. The
requirements document repeats that as the first entry in its list of consumers
and leaves the second entry unfilled, pending the program's questions about who
each piece is for.

Until 2026-08-16 this relationship was written down nowhere. The blueprint
records why that mattered in practice: the archie design threads answered on
2026-07-26 were answered by an agent that did not know remargin was the consumer,
because no document said so.

## The boundary Eduardo drew

**Decided.** On 2026-08-25 Eduardo described the boundary from his side, and the
vision blueprint's design section records where he placed each responsibility.
The blueprint is explicit that this is stated intent and not yet a design, and
keeps the mechanics open.

archie holds:

- the linter, with shapes the user specifies for a whole markdown document or for
  a sub-part of one;
- schemas with a strict and a loose mode, and no stripping mode;
- the markdown document and the schema as abstractions independent of any file on
  disk;
- a transactional change model in which every change is an object;
- version tracking abstracted over pluggable storage backends.

remargin holds:

- the resource types;
- the actions declared on each resource type — Eduardo's example is a `task` whose
  actions are `defer`, `close`, and `assign` — performed through configured
  actions;
- the write path through which every change becomes a transaction, with no write
  bypassing it.

The blueprint keeps one signal from that conversation visible: Eduardo says
"remargin schemas" and "remargin resource types" while placing the linter and the
schema engine in archie. The user-facing concepts are remargin's; the engine is
archie's. Each of the five engine capabilities has its own page — see
[Documents and schemas](documents-and-schemas.md),
[Shapes, schemas, and resource types](shapes-schemas-resource-types.md),
[Changes and versions](changes-and-versions.md), and
[Backends and capabilities](backends-and-capabilities.md).

## Resource types moved from archie to remargin

**Decided.** Resource types are remargin's, not archie's. archie holds only, in
Eduardo's words, "ways to help remargin achieve this".

The corpus records him arriving at that in two steps, in a single thread on the
root design document's purpose list. On 2026-08-16 he said resource types can be
defined in archie: the set listed in the workspace analysis was extracted from his
own files, another vault would define a different set, and one type has to be able
to extend another to add more to it. On 2026-08-30 he reversed it — he had thought
about it again and wanted resource types as part of remargin, because resource
types are "too much of business logic" for archie to hold. The root design's
purpose list now carries the later position and marks it as superseding the
earlier one, and the requirements document lists owning resource types as a stated
non-goal.

What survived the reversal is the requirement on archie's means: whatever archie
supplies has to let remargin define types per vault rather than pick from a fixed
list, and has to let one type extend another. Two places in the root design still
encode the older fixed model and fall away when the schema language is designed —
the `[resource_types] valid = [...]` block in its configuration section and the
`ResourceType` enumeration in its type sketch.

## Permissions on files and on parts of a document

**Decided.** remargin is to have a permissions system in which users hold view or
edit access to files and even to parts of markdown documents, so that remargin can
show a reader only the portion of a document they have access to. Eduardo stated
this on 2026-08-31, in the planning document's thread about what he had been
calling "the security layer", and added "this is what it's all about". It is filed
in his words in the remargin feature ledger, with the status `idea`.

**Decided.** What exists today in remargin is a path-level permission model:
restricting a path, unprotecting one, denying particular operations, and trusted
roots whose own restrictions are respected recursively. Nothing below the level of
a whole file is implemented, and before Eduardo stated the plan, no document in
the corpus described sub-file access at all.

**Proposed.** archie's contribution to it is that a part of a document has to be
addressable before access can be granted to that part. The requirements document
holds "parts of a document as addressable units of access" as a candidate goal,
and the grammar document lists it among the things that hang on the path
expression: what a path can address decides what a permission can be attached to.

**Open in the corpus.** How archie serves that mechanically is not designed
anywhere. The remargin feature ledger says so in as many words.

## What archie would do for ProDoctivity

**Proposed.** ProDoctivity is one of Eduardo's clients, and the corpus records
him wanting to hold its documents as markdown with frontmatter so that a language
model can be handed a document's text instead of an image of it. Asked how archie
helps there, he answered with a four-item list prefaced by "probably by": linting,
enforcing schemas, helping with the security layer, and helping maintain
referential integrity somehow. It is recorded as a candidate list rather than as
settled work, because he said "probably".

Three of the four already map onto things this book describes: linting and schema
enforcement are the linter and the schema engine; the security layer is the
permission system above; referential integrity is link protection together with
the structural operations below. Everything else about that use case was declared
outside archie's concern by Eduardo himself — how documents become markdown (if it
happened at all, ProDoctivity would write them through remargin), whether an
export pipeline exists, and which frontmatter fields matter to a language model.

## Structural operations: the one mechanism on record

**Decided.** Structural operations on a corpus — move, rename, merge, archive — as
integral transactions that keep links valid by construction are archie's
territory. Eduardo confirmed it, and the vision blueprint records it as the one
piece of concrete evidence it has toward what "core of remargin" means
mechanically.

The evidence came from a real reorganization on 2026-08-16. remargin's move
operation preserved its own metadata, because comment threads live inside the
files that move, but it knew nothing about links between files: every inbound
reference had to be hunted down and rewritten by hand, with judgment calls the
tool could not make about which spellings to rewrite and which to leave as
historical record. That intelligence lived in an agent improvising around the
tool, and the capability the capture draws from it is that archie owns the corpus
as a structured graph, so a move rewrites every inbound link as part of the same
transaction. The full capture is on the
[Changes and versions](changes-and-versions.md) page.

## What stays open

**Open in the corpus.** What "archie becomes the core of remargin" means
mechanically is the vision blueprint's second unanswered question. Two halves are
open. The packaging: whether remargin adds archie as the crate that provides its
document, frontmatter, and schema engine, whether remargin's file layer is
rewritten on archie's model, or whether archie absorbs remargin outright. And the
division of subsystems: which of remargin's current parts — threaded comments,
identities and signatures, permission realms, the Model Context Protocol server,
the command line, the Obsidian plugin — stay remargin's and which become archie's.

**Open in the corpus.** Two further questions from the blueprint bear directly on
this boundary. Where archie's design should start, given that remargin is the
consumer: the archie documents name a small `archie query` command over
frontmatter as the first thing to build, and whether the remargin-core goal
replaces that with a schema and frontmatter engine designed around remargin's
interface from the outset is unanswered. And what becomes of archie's
vault-application features — classifying inbox captures with a language model, and
importing from Google Tasks — whether they live inside archie, inside a separate
vault-focused application built on the engine, or are dropped.

One observation the corpus records without drawing a conclusion from it: of the
thirteen commands Eduardo wrote down on 2026-08-30, seven verbs — `ls`, `get`,
`lint`, `search`, `write`, `rm`, and `query` — are the names of operations
remargin already has, while `link` and `init` are not. Whether archie's command
surface is meant to mirror remargin's operation set at the engine level is part of
the open boundary question, and the corpus does not answer it.

> **Suggestion (documentation author):** the corpus leaves a collision here that
> someone will hit. Eduardo said in chat on 2026-08-31 that archie exposes a Model
> Context Protocol server "just like remargin", while the fate of remargin's own
> Model Context Protocol server is listed among the subsystems whose ownership is
> undecided. Eduardo would have to decide whether the system ends up with two
> servers — one for the engine and one for the collaboration layer, each with its
> own tools — or one server whose tools come from both layers. Nothing in the
> design corpus addresses this, and no page in this book assumes an answer.

Both this page's gaps and every other one are collected on the
[Design status](../design-status.md) page.
