# Introduction

**Status:** Mixed — see markers.

This book documents archie. archie does not exist as a program yet: it is being
designed first and written down second, and this book is the written form of the
design. Everything on these pages comes from the design corpus kept in Eduardo
Burgos's notes, or from a statement of his recorded with the date he made it.
Nothing here is invented; where the design is silent, the page says so.

## What archie is

**Decided.** archie is a tool for validating and maintaining the structure of a
markdown corpus. The root design document opens with exactly that, and adds that
Eduardo's Obsidian vault is the first corpus archie serves — a first user, not a
requirement. He restated the second half on 2026-08-30: nothing about archie will
require Obsidian, and that non-goal is recorded in the requirements document.

**Decided.** archie is written in Rust, and it is an engine before it is a
program. The vision blueprint records the consequence of decision D1 (below) in
those terms: archie's engine layers are designed as a library first. On
2026-08-16 Eduardo chose the project shape that follows from it — a Cargo
workspace holding library crates with a thin command-line crate on top, recorded
as the decision on the frontmatter and architecture discussion. Which crates the
workspace holds is not settled; the crate sketch in that document predates the
direction Eduardo set on 2026-08-25 and is to be re-derived.

**Decided.** archie reads and writes markdown with a parser of its own, covering
both the frontmatter and the document body. Eduardo settled this on 2026-08-30
and 2026-08-31 on the planning document: existing crates may inspire it, but the
parser is archie's own code, it includes the frontmatter parser and the markdown
parser, and he requires it to be a good one. He will state performance goals for
it; until he does, no performance claim appears in this book.

**Decided, stated outside the design corpus.** archie exposes a Model Context
Protocol (MCP) server, just like remargin. Eduardo said this in chat on
2026-08-31. No document in the design corpus records it yet, so this book carries
the statement with its date, and the gap is listed on the
[Design status](design-status.md) page. What that server exposes is documented in
the [Model Context Protocol server](reference/mcp/index.md) reference.

So archie has three faces over one engine: the Rust library, the command line,
and the Model Context Protocol server.

## archie becomes the core of remargin

**Decided.** Eduardo stated on 2026-08-16 that archie will become the core of
remargin, and that this is the reason archie is being built. It is recorded as
decision D1 in the vision blueprint of the Remargin Vision program. remargin is
his working document-collaboration system — the tool he uses daily, paired with
Obsidian. The consequence the blueprint draws from D1: archie's primary consumer
is remargin rather than any one person's notes.

**Proposed (stated by Eduardo on 2026-08-25).** The blueprint's section on the
design records where Eduardo placed each responsibility when he described the
boundary that day. archie holds the linter with user-specified shapes, the
schema engine with its strict and loose modes, the markdown document and the
schema as abstractions independent of files, a transactional model in which
every change is an object, and version tracking abstracted over pluggable
storage backends. remargin holds the resource types, the actions declared on
each resource type, and the write path through which every change becomes a
transaction. The blueprint records that whole placement as stated intent and not
yet a design, and the feature ledger holds each entry with the status `idea`, so
every one of those capabilities is Proposed in this book — see
[archie and remargin](concepts/archie-and-remargin.md) .

**Open in the corpus.** What "archie becomes the core of remargin" means
mechanically is not settled — whether remargin depends on archie as a crate,
whether remargin's file layer is rewritten on archie's model, or something else,
and which of remargin's current subsystems stay remargin's. The blueprint holds
this as an open question and this book does not answer it. The
[archie and remargin](concepts/archie-and-remargin.md) page collects what is
known.

## There is no binary yet

**Decided.** archie is in its design phase and no code has been written. The
vision blueprint recorded on 2026-08-16 that the archie repository holds no code,
only its task-tracking configuration, and the grammar document repeated it on
2026-08-31. The repository is `eburgos/archie` and it is public (Eduardo,
2026-08-30, confirmed against GitHub the same day).

Every command example in this book therefore describes an intended command line,
not one that has been run. Expected output appears only where the design
specifies it.

## How to read the status markers

Every normative statement in this book carries one of three markers, and every
page carries a status line under its heading.

- **Decided** — recorded as settled in the design corpus. The prose names the
  document, and the thread or section, that settled it. Two things qualify: a
  numbered decision in the program's vision blueprint, and a point a comment
  thread records as settled.
- **Proposed** — present in the corpus but marked as a strawman, a candidate, or
  an idea. It is written down; it is not agreed. A direction Eduardo stated whose
  only record is a feature-ledger entry with the status `idea` together with a
  candidate goal in the requirements document is written as **Proposed (stated by
  Eduardo on _date_)**, which credits him with the direction without claiming it
  was accepted as a goal. The five capabilities he placed in archie on 2026-08-25
  are all of that kind.
- **Suggestion** — an addition by the author of this documentation that the
  design does not contain. It is rendered as a blockquote that opens with
  **Suggestion (documentation author):**, and it says what Eduardo would have to
  decide. It is never presented as fact.

A page's status line reads **Status:** Decided, Proposed, or Mixed. A page marked
**Status:** pending is a placeholder whose content has not been written yet.

## How this book is organized

- **Tutorial** — [A first session with archie](tutorial/first-session.md), one
  walk from an empty corpus to a change, front to back.
- **Guides** — how to accomplish one task, one guide per task.
- **Reference** — the command line, the Model Context Protocol server, and the
  languages and formats: shapes, the query grammar, validation rules, change
  objects, configuration, output formats, and exit codes.
- **Concepts** — the ideas the rest of the book leans on, including where the
  boundary between archie and remargin runs.
- **Status** — the [Design status](design-status.md) page: what is decided, what
  is proposed, which suggestions this book adds, which questions the corpus
  leaves open, and where the documentation and the corpus do not yet meet.

Maintainers of this book should also read `../README.md`, which holds the
conventions these pages follow, the map of the design corpus, and the inventory
of every command the corpus names.
