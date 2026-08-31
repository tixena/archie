# Design status

**Status:** Mixed — see markers.

This page is the index of how settled archie's design is. It exists so that a
reader can tell, without reading every page, which parts of this book describe
agreements and which parts describe ideas that are still being argued over.

Four of the five sections below are filled by the pass that closes this book once
the other pages are written. The open questions are listed now, because they are
what every other page has to defer to.

## Decided

_To be filled by the final documentation pass: every statement in this book
marked **Decided**, with the document and thread that settled it._

## Proposed

_To be filled by the final documentation pass: every statement in this book
marked **Proposed**, with the document that holds it and the reason it is not
settled._

## Suggestions

_To be filled by the final documentation pass: every addition this book makes
that the design corpus does not contain, each with the decision it asks Eduardo
for._

## Open questions in the corpus

These are the questions the design corpus itself records as unanswered. They are
Eduardo's to settle. Until they are settled, no page in this book may answer them.

### From the program's vision blueprint

- **What each component of the system is for, and which tools are members of it.**
  Five members are confirmed: remargin, archie, vigil, the methodology named
  marginalia, and a user-interface product provisionally called Remargin UI. What
  is missing is a one-sentence role for each in Eduardo's own words, whether the
  task tracker, the component-grafting tool, and the notes vault itself are
  members or merely adjacent tools, the final name for the user-interface
  product, and how that product relates to the Obsidian-plugin work already
  recorded in remargin's corpus.
- **What it means, mechanically, for archie to become the core of remargin.** The
  relationship is settled; the mechanism is not. Whether remargin adds archie as
  the crate that provides its document, frontmatter, and schema engine, whether
  remargin's file layer is rewritten on archie's model, or whether archie absorbs
  remargin outright — and which of remargin's current subsystems (threaded
  comments, identities and signatures, permission realms, the Model Context
  Protocol server, the command line, the Obsidian plugin) stay remargin's and
  which become archie's.
- **Who each piece is for, and what becomes public.** For every component:
  personal tooling, Tixena-internal, or a public product? Which repositories are
  published, and under which account or organization?
- **The role of the notes vault in the system.** Whether `eburgos_notes` is the
  reference deployment whose structure the tools are built to serve, or one
  instance among many that the tools must stay generic for. The answer decides
  how much of that vault's philosophy hardens into archie's defaults.
- **What the marginalia methodology is beyond internal practice.** Whether it is
  only how the program works internally, or also something shipped, taught, or
  published alongside the tools. The answer decides how audience-facing its
  documents have to become.
- **Where archie's design should start, given that remargin is its consumer.**
  The archie documents name a vault-linting walking skeleton — a small
  `archie query` command — as the starting point. Whether the remargin-core goal
  changes that, so that the schema and frontmatter engine is designed around
  remargin's interface from the outset with the command line as a thin shell, is
  open.
- **What becomes of archie's vault-application features.** The root design
  includes classifying inbox captures with a language model and importing from
  Google Tasks. Whether those live inside archie, inside a separate
  vault-focused application built on the engine, or are dropped, is open.
- **Whether the program is also a platform for building small systems.** With
  resource types carrying declared actions of a known shape, the user interface
  could render any resource with its actions and agents could be allowed to
  execute them. Eduardo flagged this on 2026-08-25 as an idea he would like to
  explore, not a commitment. What counts as such a system, where the boundary
  between the named products and a general platform lies, and what an agent is
  permitted to do when it executes an action, are all open.

### From the grammar document

- **One expression language or two.** Whether `archie query`, `archie search`,
  and the path expression given to `archie write` share a single grammar, so that
  a path is a query selecting inside one document, or whether the document-set
  query and the within-document path are two grammars joined at a defined seam.
  Eduardo's answer on 2026-08-31: "I honestly don't know."
- **What a path expression can address.** Whether a path selects by heading
  level, by heading title, and by an index among children, and whether it can also
  reach list items, paragraphs, tables, code fences, remargin comment blocks, and
  frontmatter keys.
- **What a projection produces.** Markdown fragments, frontmatter values,
  file paths, or a mix chosen per clause — and which of those is the default.
- **Which mutation verbs the language carries, and where a transaction ends.**
  Appending is the only verb Eduardo has written down. Whether the language also
  carries setting, removing, replacing, and moving is open, as is whether one
  query-driven mass modification is a single change object or one change object
  per document touched.
- **How search composes with query.** Whether search is a separate verb with its
  own clause syntax or a query with body matching as its default stage, and what
  the short flags in Eduardo's search example mean.
- **The performance goals for archie's parser.** Eduardo requires the parser to
  be a good one and will state the goals; until he does, nothing about the parser
  is testable.
- **The spelling of the language itself.** Operators, quoting, escaping, and how
  a path is written inside a pipeline as opposed to as a command-line flag.
  Nothing is written down beyond the four expressions Eduardo has typed.

### Recorded elsewhere in the corpus

- The root design leaves two questions of its own open: how authentication
  against Google Tasks is handled, and whether one configuration may cover more
  than one vault.
- The mechanism by which a file takes on properties from the folders above it is
  parked. Eduardo said on 2026-08-16 that something like it is wanted but that he
  is not convinced the `inherits:` block in a folder's `index.md` is the way, and
  asked to discuss other aspects first.

## Gaps between the corpus and this documentation

_To be filled by the final documentation pass: everywhere this book had to say
something the corpus does not say, or had to leave a page thin because the corpus
is silent. The one already known: archie exposing a Model Context Protocol (MCP)
server is a statement Eduardo made in chat on 2026-08-31 and no design document
records it._
