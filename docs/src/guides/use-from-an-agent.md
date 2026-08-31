# Use archie from an agent

**Status:** Mixed — see markers.

An agent — a program driven by a language model — does not open a terminal. It
calls tools. archie is to expose a Model Context Protocol (MCP) server for exactly
that: the Model Context Protocol is the interface through which a model-driven
program discovers the tools a system offers and calls them. This guide covers what
is known about using archie that way, which is less than a reader would like, and
it is careful throughout about which sentences describe archie and which describe
remargin.

## What is decided, and where it was said

**Decided, stated outside the design corpus.** archie exposes a Model Context
Protocol server, just like remargin. Eduardo Burgos said so in chat on
2026-08-31. No document in the design corpus records it — not the root design, not
the planning document, not the program's vision blueprint — so this book carries
the statement with its date and lists the gap on the
[Design status](../design-status.md) page.

Everything that follows from that one statement is the subject of the
[Model Context Protocol server](../reference/mcp/index.md) reference and the
[Model Context Protocol tools](../reference/mcp/tools.md) page beside it. This
guide is the task-shaped view: what an agent would do, in what order.

## Step 1: know what you are calling

archie has three faces over one engine — a Rust library, a command line, and a
Model Context Protocol server. An agent uses the third. The command line and the
server are two ways into the same engine, which is why every capability described
in this book's reference section is the subject matter of both, and why nothing in
the server can be more settled than the engine underneath it.

That matters more than usual here, because most of archie's surface is unsettled.
The verbs Eduardo wrote down on 2026-08-30 are explicitly not written in stone,
the expression language they carry is not designed, and the shape language the
linter checks against does not exist. An agent calling archie today would have
nothing to call.

## Step 2: expect remargin's conventions

Eduardo's statement was "just like remargin", so remargin's conventions are the
ones archie's server is expected to mirror. What follows is a description of
remargin's server, observed from the tools it publishes. Each point is a fact
about remargin and not a statement about archie.

- **Flat, verb-named tools.** One tool per operation, named for the operation —
  reading a file, listing a directory, searching text, querying comments, writing,
  editing, moving, copying, removing — rather than one tool with a mode argument.
- **Compact payloads with the column names stated once.** A result that carries
  many similar rows names the columns in one list and then emits each row as a
  positional array, so a field name is transmitted once rather than once per row.
  A search result, for instance, carries the column names, then the matching files,
  and under each file its matches as rows.
- **Paging that never hides a truncation.** A caller may ask for a page with a
  limit and an offset, and the result carries the exact total for the whole corpus
  regardless of the page size, plus an explicit marker when a page was clamped. So
  a bounded page is never mistaken for a complete answer.
- **A projection step before anything mutates.** A separate call takes the
  operation a caller intends and reports what it would do — what would change,
  what would be rejected, and whether the operation would do nothing at all —
  without touching disk.
- **An atomic batch.** Several writes to one document go in one call that either
  applies as a whole or not at all, rather than as a sequence of independent calls.
- **An identity call.** The server answers who it is acting as, so an agent can
  find out rather than assume.
- **Binary content described as well as delivered.** Non-text content comes back
  with its type and size alongside the bytes, and a caller is expected to check
  the size before pulling a large one.

> **Suggestion (documentation author):** what archie's tools are actually called
> is not written anywhere, and this book does not name them. Three decisions would
> settle it and all three are Eduardo's. **The tool set:** whether the server's
> tools mirror the command-line verbs one for one — a tool per verb for listing,
> retrieving, querying, searching, writing, removing, linking, initializing, and
> linting — or whether the server exposes the engine's operations directly and the
> command line is the layer that arranges them into verbs. The program's vision
> blueprint keeps that question open in a wider form, asking whether the engine
> should be designed around remargin's interface from the outset with the command
> line as a thin shell over it. **The preview:** whether archie adopts remargin's
> projection step, which matters more for archie than for remargin because every
> archie write is a change object that can be replayed and unapplied, so a caller
> could be shown the change object a write would produce rather than a summary of
> it. **The boundary with remargin's own server:** the
> [archie and remargin](../concepts/archie-and-remargin.md) page records that
> collision — whether the system ends with two servers or one — and this guide
> assumes no answer to it.

## Step 3: reading a corpus

The read-side operations the corpus names are listing the contents of a corpus,
retrieving a resource by path — a folder, a markdown file, an image, or a document
in Portable Document Format — selecting documents by an expression over their
frontmatter, and matching a pattern against document bodies and frontmatter. Each
has its own reference page and each is Proposed: see
[archie ls](../reference/cli/ls.md), [archie get](../reference/cli/get.md), and
[Query and search](query-and-search.md).

For an agent, one of those matters more than the rest. Retrieving a document as
parsed data rather than as text is what makes a corpus usable as context for a
model without the model having to parse markdown itself — and what a retrieval
returns for a markdown file, the raw bytes or the parsed document, is one of the
things the corpus does not state.

## Step 4: changing a corpus

**Decided.** Every write is a transaction that produces a change object, which can
be replayed to a byte-identical result and unapplied in reverse. An agent's write
is not an exception: Eduardo's statement of 2026-08-25 covers "ANY CHANGE". See
[Change objects](../reference/change-objects.md) and
[Write through paths](write-through-paths.md).

**Decided.** What an agent may see and change is meant to be bounded. Eduardo
stated on 2026-08-31 that remargin is to have a permission system in which people
hold view or edit access to files and to parts of markdown documents, so that only
the accessible portion of a document is shown. archie's contribution is that a
part of a document has to be addressable before access to it can be granted or
refused, and how archie serves that mechanically is not designed.

## What an agent gets from archie's design today

One thing, and it is not a server. The root design document names a language-model
agent as an audience for the command that serializes a corpus: it emits, for each
file, that file's frontmatter, the categorized observations found in its body, the
typed relations to other resources, and a graph of nodes and edges built from those
relations, all in JavaScript Object Notation (JSON). That command's own fate is
open — it is absent from Eduardo's command list, and the vision blueprint asks what
becomes of archie's features for maintaining a personal corpus. See
[archie export](../reference/cli/export.md) and
[Knowledge patterns](../concepts/knowledge-patterns.md).

The other place a language model appears in archie's design is the inbox flow,
which asks a model to classify unfiled captures. That is archie using a model, not
an agent using archie, and its fate is open in the same way. See
[archie inbox](../reference/cli/inbox.md).

## What is not decided

- The server itself: how it is started, how it is addressed, and what it is called.
- Every tool name, parameter, and result shape.
- Whether the system ends up with one Model Context Protocol server or two.
- Whether the command line or the server is archie's primary interface, which the
  vision blueprint holds open.

## Related pages

- [Model Context Protocol server](../reference/mcp/index.md) — the server
  reference.
- [Model Context Protocol tools](../reference/mcp/tools.md) — the tools it
  exposes.
- [archie and remargin](../concepts/archie-and-remargin.md) — the boundary
  between the two systems, including the two-server question.
- [Write through paths](write-through-paths.md) — what a write commits.
- [Query and search](query-and-search.md) — the read operations an agent would
  call.
- [Design status](../design-status.md) — where this page's central fact is
  recorded as a gap.
