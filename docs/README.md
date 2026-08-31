# archie documentation — maintainer guide

**Status:** Mixed — see markers.

This folder holds the archie book: an mdBook whose pages are written from
archie's design corpus and from nothing else. archie has no code yet, so this
book describes a tool that is being designed, not one that has been built. This
page holds the rules the book follows, the map of where its content comes from,
and the inventory of every command the corpus names.

## Building the book

```console
$ mdbook build docs
$ mdbook serve docs
```

`mdbook build docs` writes the rendered book to `docs/book/`, which is ignored by
git. `mdbook serve docs` serves it and rebuilds on change.

The gate that has to pass before any documentation change is finished:

```console
$ bash scripts/docs-check.sh
```

It builds the book and then checks the structure of the source pages. What it
enforces is listed under "The gate" below.

## Where the content comes from

The design corpus lives in Eduardo Burgos's notes vault, under
`remargin_vision/`. It is **read-only for this book**: it is managed by remargin,
it is read through remargin's tools, and it is never edited and never commented
on from here. Decisions live in the comment threads as much as in the document
bodies, so both are read.

| Document | What it holds |
|---|---|
| `remargin_vision/index.md` | The program's vision blueprint. It records that archie will become the core of remargin, the boundary Eduardo drew on 2026-08-25 between what archie holds and what remargin holds, and the list of questions the program has not answered. |
| `remargin_vision/archie/archie.md` | The root design document: overview and purpose, the original command set, the validation-rule catalog, knowledge patterns, the configuration file, output formats, the crate layout, exit codes, and the design decisions resolved so far. Its body was rewritten in review rounds on 2026-07-26 and on 2026-08-30 and 2026-08-31; the current body plus its threads is what counts. |
| `remargin_vision/archie/reorganize_workspace.md` | The analysis of Eduardo's own vault: the resource types extracted from it, how a folder's `index.md` declares inherited properties and permitted types, and the philosophy of a global inbox with per-area archives. |
| `remargin_vision/archie/requirements.md` | The requirements skeleton: the consumer order, candidate goals awaiting Eduardo's acceptance, the two stated non-goals, and a placeholder for the parser's performance goals. |
| `remargin_vision/archie/discussions/archie_grammar.md` | The mini request-for-comments on the query, path, and search grammar. Its design section is a strawman, so everything in it is Proposed; its final section lists what is still unspecified. |
| `remargin_vision/archie/discussions/archie_planning_next_steps.md` | The planning document. Its threads carry most of the recent decisions: archie's own parser including the frontmatter and markdown parsers, archie respecting other people's formatting while remargin writes its canonical form, the repository being public, the jq-like language with chaining, projection, and mutation, `link` generalized from the `aa_tool` implementation, "the security layer" turning out to mean permissions on files and on parts of markdown documents, and Eduardo's own list of the commands he would run. |
| `remargin_vision/archie/discussions/archie_frontmatter_discussion.md` | The frontmatter engine as archie's foundation, and the choice between one crate and a Cargo workspace of library crates. |
| `remargin_vision/archie/discussions/2026-08-16__structural_operations.md` | Two capabilities captured from a real vault reorganization: moves that keep every inbound link valid as one transaction, and a merge-move that unions into an existing destination instead of replacing it. |
| `remargin_vision/archie/discussions/archie_ai_context_vision.md` | A capture of what Eduardo said about storing markdown-with-frontmatter as context for a language model, and what it settles for archie. It claims nothing beyond that. |
| `remargin_vision/features/archie.md` | Eduardo's own words on what archie holds, one section per idea: the linter with user-specified shapes, strict and loose schema modes with no stripping, transactional change objects that replay to a byte-identical document and can be unapplied, the document and the schema as abstractions, backend-agnostic versioning, and `link`. |
| `remargin_vision/features/remargin.md` | Eduardo's own words on what remargin holds: resource types enforced by archie's linter, actions declared per resource type, every write as a transaction, reply schemas per comment kind, and permissions on files and on parts of markdown documents. |
| `~/src/tixena/aa_tool/src/link.rs` | The implementation that archie's `link` generalizes. Ordinary source code, readable directly. |

One fact this book states is not in the corpus at all: archie exposes a Model
Context Protocol (MCP) server, just like remargin. Eduardo said so in chat on
2026-08-31. Every page that mentions it cites that statement and its date, and it
is listed as a gap on the book's design-status page. archie's server should
mirror remargin's conventions — flat verb-named tools, compact payloads whose
repeated rows are positional and named once by a columns list, paging that always
reports the exact total, and a preview step for operations that mutate.

## Status markers

Every normative statement in the book carries one of three markers.

- **Decided** — recorded as settled in the corpus. Name the document, and the
  thread or section, in the prose.
- **Proposed** — in the corpus, but marked as a strawman, a candidate, or an
  idea.
- **Suggestion** — the documentation author's own addition, absent from the
  design. Render it as a blockquote that begins
  `> **Suggestion (documentation author):**`, say what Eduardo would have to
  decide, and never present it as fact.

Every page carries a status line directly under its heading: `**Status:**`
followed by Decided, Proposed, or Mixed — see markers. A page that has not been
written yet carries `**Status:** pending`.

## Writing rules

- Plain, self-sufficient prose. A reader never has to look something up to
  understand a sentence.
- Expand any shorthand on its first use on each page, for example "Model Context
  Protocol (MCP)".
- No issue-tracker identifiers anywhere, and no note of which tool or model wrote
  a page.
- No release framing. Do not write "v1", "MVP", or "first release"; Eduardo has
  asked for those words to disappear from his projects.
- No hype words and no speculation words. Every claim is traceable to a source or
  is marked as a Suggestion.
- When the corpus is silent, say so on the page. Do not fill the gap.
- mdBook markdown: no frontmatter block at the top of a page, because mdBook
  renders it as body text. Exactly one level-one heading per page. Links to
  sibling pages are relative.
- Command examples are console blocks whose lines begin with `$ archie`. Show
  expected output only where the design specifies it — the root design's
  output-format section is the one place that does — and otherwise show the
  command line alone.
- Each page is owned by exactly one task. Do not edit a page another task owns,
  and do not add entries to `src/SUMMARY.md`, which the scaffold owns.

## The gate

`scripts/docs-check.sh` fails unless all of the following hold.

1. Every chapter path listed in `src/SUMMARY.md` exists on disk.
2. `mdbook build docs` completes without an error.
3. No page under `src/` begins with a frontmatter fence.
4. Every page under `src/` has exactly one line starting with `# `, counting only
   lines outside fenced code blocks.
5. Every page under `src/` has a line starting with `**Status:**`.
6. Every relative link to a markdown file, from any page under `src/`, resolves
   to a file that exists; the anchor part of a link is ignored.

`src/SUMMARY.md` is exempt from checks 3 to 5: it is the table of contents, not a
page — its part headings are level-one headings by mdBook's own convention, and
it makes no claims that need a status.

The script prints one `PASS` line with the number of pages it checked.

## Command inventory

This is the list of every command the design corpus names, and it is the anchor
the rest of the book stays consistent with. A command's status is one of:

- **Decided** — settled in the corpus.
- **Proposed** — written down, not agreed.
- **Root-design-only, fate open** — it appears in the root design document, it is
  absent from the list Eduardo wrote on 2026-08-30, and the program's vision
  blueprint holds an open question about whether archie's vault-application
  features live in archie, live in a separate application built on the engine, or
  are dropped.

### The commands Eduardo named on 2026-08-30

He wrote these as the things he would actually run, on the planning document. On
2026-08-31 he added that the list is not written in stone — the verbs and the
syntax can change — and that statement is itself recorded as a settled point on
the grammar document. So every command below is **Proposed**, even where
something about its behavior is Decided.

| Command | What the corpus says | Status |
|---|---|---|
| `archie ls` | Lists the contents of a corpus. No behavior is described anywhere. | Proposed |
| `archie get <path>` | Retrieves a folder, a markdown file, an image, or a PDF. | Proposed |
| `archie lint <path>` | Checks documents against shapes. It stands where the root design has `check`. | Proposed |
| `archie link <path>` | Turns mentions of a resource into links to that resource. | Proposed |
| `archie init <path>` | Sets up a corpus. The root design gives it the job of creating the configuration file and any missing folder index files. | Proposed |
| `archie search <path> <pattern>` | Matches a pattern against documents. | Proposed, with a Decided scope |
| `archie write <file> --path=<expr>` | Writes into the part of one document that a path expression addresses. His example appends content taken from another file. | Proposed |
| `archie rm <path>` | Removes a file or a folder. No behavior is described anywhere. | Proposed |
| `archie query --exp=<expr>` | Selects documents by an expression over their frontmatter. | Proposed, with a Decided starting point |

What is Decided about individual entries above:

- **`archie query`.** That a frontmatter query is the first thing archie will do
  is settled: Eduardo confirmed on 2026-08-16, on the frontmatter discussion,
  that `archie query 'tags contains "rule"' .grit/patterns/` is what gets built
  first, and that he would give the signal to start. He has not given it. The
  direction of the language is settled too, on the planning document (2026-08-16,
  restated 2026-08-30): something like `jq`, with queries that chain and with
  almost anything projectable, and with the same language driving modification,
  one document at a time or in bulk — his example being to append a section
  carrying the current date to every element a query returns. The grammar itself
  is not designed; the grammar document holds a strawman and a list of what is
  unspecified.
- **`archie search`.** Its scope is settled (2026-08-30, planning document,
  recorded as a decision on the grammar document): search runs over document
  bodies, over frontmatter when frontmatter is specified, and over both when
  clauses are given for both. Eduardo named this as one of his reasons for
  writing archie's own parser. The short flags in his example are not defined.
- **`archie link`.** Eduardo pointed at the `link` subcommand of his `aa_tool`
  project as almost the same behavior, and said on 2026-08-31 that in archie it
  is not restricted to contacts — it can work for any resource. The rest waits
  for a feature discussion. The feature ledger records the entry with the status
  `idea`. What `aa_tool` does, read from its source: it wraps every
  collision-free mention of a person's name or alias in a relative-path link to
  that person's profile, deterministically and additively, never touching
  frontmatter, fenced blocks, inline code, existing links, or raw URLs, and it
  has an exact inverse that strips only the links it would have produced.
- **`archie get`.** That his examples reach for an image and a PDF makes
  non-markdown files retrievable resources, which the root design touched only
  through its rule about assets nothing references. The requirements document
  holds it as a candidate goal, not an accepted one.

### The commands that exist only in the root design

These come from the root design's command section. None of them appears in
Eduardo's 2026-08-30 list, and what becomes of them is one of the program's open
questions.

| Command | What the corpus says | Status |
|---|---|---|
| `archie check [PATH]` | Validates a corpus or one path within it, with options to choose the output format, the severity that makes the run fail, which rules to run, and which patterns to ignore. Eduardo's list has `lint` in this position. | Root-design-only, fate open |
| `archie fix [PATH]` | Repairs what can be repaired without a human decision, with a dry run, a rule filter, and an interactive mode. The root design's list of what it can and cannot repair encodes the older hard-coded model and is deferred until the schema language is designed. | Root-design-only, fate open |
| `archie inbox [PATH]` | Classifies unprocessed captures with a language model and proposes where each one should be filed, for a person to approve, edit, skip, or reject. The vision blueprint names this feature explicitly when it asks what becomes of archie's vault-application features. | Root-design-only, fate open |
| `archie trace <FILE>` | Prints the chain of inherited properties for one file, from the root down to the file, showing what it must carry and what it does carry. The inheritance mechanism it displays is itself parked. | Root-design-only, fate open |
| `archie export [PATH]` | Serializes a corpus for another program to read: each file's frontmatter, its observations and relations, and a graph of the links between resources. | Root-design-only, fate open |
| `archie import <SOURCE>` | Pulls items in from an external source, with Google Tasks as the one worked example. The vision blueprint names this feature too. | Root-design-only, fate open |
| `archie help` | Prints help. The root design lists it with the others; it gets no page of its own in this book. | Root-design-only, fate open |

### Structural operations

No command spelling exists for these, so the book documents them on one page
rather than one page per verb.

- **Decided** that they are archie's territory. The vision blueprint records that
  Eduardo confirmed it: structural operations on a corpus — move, rename, merge,
  archive — as integral transactions that keep links valid by construction,
  rather than validating and repairing afterwards.
- **Proposed** in every detail. The capture that recorded them says the semantics
  get designed later: which spellings of a reference get rewritten and which are
  preserved, how the operation reports what it rewrote, and what a dry run does.
  It also describes a merge-move that unions into an existing destination under
  an explicit collision policy, removes a source folder once it is emptied, and
  moves nothing at all if any entry is refused.

### Spellings that were withdrawn or superseded

Recorded so that no later page revives them.

- The shape of `archie query` as named flags plus a `--where` expression
  (`archie query --type project --status active`) was a recommendation made on
  the planning document by an agent. It is explicitly withdrawn in favor of the
  jq-like language.
- The field-editing commands sketched on the planning document — setting a field,
  removing a field, adding to an array, and a bulk edit driven by `--where` — are
  agent-written options, not Eduardo's words. Under the decided direction, the
  query language itself drives modification.
- `archie init-indexes`, in the workspace analysis, is an older spelling of the
  job the root design gives to `archie init` with its index-creating option.
