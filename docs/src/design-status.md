# Design status

**Status:** Mixed — see markers.

This page is the index of how settled archie's design is. It exists so that a
reader can tell, without reading every page, which parts of this book describe
agreements and which parts describe ideas that are still being argued over.

The five sections below are the whole of it: what the corpus settles, what it
holds without settling, what this book adds on its own account, what the corpus
records as unanswered, and where this book and the corpus do not meet. A reader
who wants only one of the five can stop at its heading.

Two conventions govern the first two lists, and both are stated in full in the
maintainer guide at `../README.md`. **Decided** covers a numbered decision in the
program's vision blueprint and a point a comment thread records as settled.
**Proposed** covers everything else in the corpus, including a direction Eduardo
Burgos stated whose only record is a feature-ledger entry with the status `idea`
together with a candidate goal in the requirements document — written as
"Proposed (stated by Eduardo on _date_)", which credits him with the direction
without claiming it was accepted as a goal.

## Decided

Thirty statements in this book are Decided. Each line gives the statement, where
the corpus settles it, and the pages that carry it.

### What archie is, and where it sits

1. **archie will become the core of remargin, and that is why archie is being
   built.** The vision blueprint's decision D1, stated by Eduardo on 2026-08-16.
   [Introduction](introduction.md),
   [archie and remargin](concepts/archie-and-remargin.md).
2. **archie validates and maintains the structure of a markdown corpus, and
   Eduardo's Obsidian vault is the first corpus it serves rather than a
   requirement.** The root design document's opening. [Introduction](introduction.md).
3. **Nothing about archie will require Obsidian.** Eduardo, 2026-08-30; recorded
   as one of two stated non-goals in the requirements document.
   [Introduction](introduction.md), [Configuration](reference/config.md).
4. **archie is written in Rust, as an engine before a program: a Cargo workspace
   of library crates with a thin command-line crate on top.** Eduardo,
   2026-08-16, on the frontmatter and architecture discussion; the consequence of
   D1 in the blueprint. [Introduction](introduction.md),
   [Documents and schemas](concepts/documents-and-schemas.md).
5. **No code has been written, and the repository `eburgos/archie` is public.**
   The blueprint on 2026-08-16, repeated on the grammar document on 2026-08-31;
   the repository question answered by Eduardo on 2026-08-30.
   [Introduction](introduction.md),
   [A first session with archie](tutorial/first-session.md).
6. **The tool is named `archie`, and its configuration file is `archie.toml`.**
   The root design's naming decision, chosen over ten other candidates.
   [Configuration](reference/config.md).
7. **The frontmatter engine is what every other feature stands on.** The
   frontmatter and architecture discussion argues it by walking the command set.
   [Documents and schemas](concepts/documents-and-schemas.md).

### The parser and the language

8. **archie writes its own parser, and that includes the YAML parser for the
   frontmatter and the parser for the markdown body.** The grammar document's
   first numbered decision, from Eduardo on 2026-08-30 and 2026-08-31; he will
   state performance goals and has not. [Introduction](introduction.md),
   [Documents and schemas](concepts/documents-and-schemas.md),
   [Query grammar](reference/query-grammar.md),
   [archie query](reference/cli/query.md).
9. **archie may respect formatting it did not write; remargin's write path emits
   the canonical form.** The grammar document's second numbered decision, from
   Eduardo on 2026-08-16 and narrowed on 2026-08-30.
   [Documents and schemas](concepts/documents-and-schemas.md),
   [Query grammar](reference/query-grammar.md),
   [archie query](reference/cli/query.md), [archie write](reference/cli/write.md),
   [Configuration](reference/config.md).
10. **Search runs over document bodies, over frontmatter when frontmatter is
    specified, and over both when clauses are given for both — and that is one of
    the reasons for archie's own parser.** The grammar document's third numbered
    decision, from Eduardo on 2026-08-30.
    [Query and search](guides/query-and-search.md),
    [archie search](reference/cli/search.md),
    [Query grammar](reference/query-grammar.md),
    [A first session with archie](tutorial/first-session.md),
    [Model Context Protocol tools](reference/mcp/tools.md).
11. **The command list Eduardo wrote on 2026-08-30 is not written in stone: the
    verbs and the syntax can change.** The grammar document's fourth numbered
    decision, from Eduardo on 2026-08-31. Every command spelling in this book
    carries the qualification.
    [Command-line interface](reference/cli/index.md),
    [Query grammar](reference/query-grammar.md).
12. **A frontmatter query is the first thing archie will do, and starting is on
    Eduardo's signal, which he has not given.** Confirmed on 2026-08-16 on the
    frontmatter and architecture discussion.
    [Documents and schemas](concepts/documents-and-schemas.md),
    [archie query](reference/cli/query.md),
    [Query and search](guides/query-and-search.md),
    [A first session with archie](tutorial/first-session.md).
13. **The language is to work like `jq` — queries that chain, almost anything
    projectable — and the same language drives modification, one document at a
    time or in bulk.** Eduardo on 2026-08-16, restated on 2026-08-30.
    [Query and search](guides/query-and-search.md),
    [archie query](reference/cli/query.md),
    [Query grammar](reference/query-grammar.md).
14. **Appending is the one mutation verb on record.** It appears in Eduardo's
    `archie write` command line and in his description of a bulk change.
    [Write through paths](guides/write-through-paths.md),
    [archie write](reference/cli/write.md).
15. **The proposal of named flags for common fields plus a `--where` expression is
    withdrawn.** It was written by an agent on the planning document and replaced
    by the language above. [Query and search](guides/query-and-search.md),
    [archie query](reference/cli/query.md), [archie rm](reference/cli/rm.md).
16. **Four expressions are the whole of what Eduardo has typed**, and the grammar
    document records that their syntax is not a hard requirement.
    [Query grammar](reference/query-grammar.md).

### Schemas, types, and the rule catalog

17. **Schemas rule both frontmatter and document structure, and whether
    frontmatter is required at all follows from the governing schema rather than
    from a fixed rule.** Eduardo, 2026-07-26, in a thread on the root design's
    purpose list. [Documents and schemas](concepts/documents-and-schemas.md),
    [Shapes, schemas, and resource types](concepts/shapes-schemas-resource-types.md),
    [Shape language](reference/shape-language.md).
18. **The rule catalog, the rule and resource-type blocks in the configuration,
    and the list of automatic repairs are deferred, and each becomes a
    consequence of what the schemas say.** The deferral note the root design
    carries from 2026-08-16; Eduardo's answer about when to rework it was "I
    don't know but as long as you can keep it in mind I feel safe."
    [Validation rules](reference/validation-rules.md),
    [archie check](reference/cli/check.md), [archie fix](reference/cli/fix.md),
    [Lint a vault](guides/lint-a-vault.md).
19. **Resource types belong to remargin, not archie; archie holds only the means
    to help remargin enforce them.** Eduardo, 2026-08-30, superseding what he
    said on 2026-08-16; recorded as a stated non-goal in the requirements
    document. [archie and remargin](concepts/archie-and-remargin.md),
    [Shapes, schemas, and resource types](concepts/shapes-schemas-resource-types.md),
    [Enforce resource types](guides/enforce-resource-types.md),
    [archie check](reference/cli/check.md),
    [Shape language](reference/shape-language.md).
20. **Those means have to allow types defined per corpus, and a type that extends
    another to add to it.** Eduardo, 2026-08-16; the half of that statement that
    survived the change of owner.
    [Enforce resource types](guides/enforce-resource-types.md),
    [Shape language](reference/shape-language.md).
21. **Knowledge patterns — observations and relations — are recognized and
    extractable.** A purpose bullet in the root design, whose meaning Eduardo
    asked about on 2026-07-26 and then acknowledged.
    [Knowledge patterns](concepts/knowledge-patterns.md).
22. **The configuration file is where lint rules may be adjusted, and that is
    all.** Eduardo, 2026-08-16: "I may allow some 'linting rules' in the config
    and that's it." [Configuration](reference/config.md).

### Inheritance, structure, and links

23. **Something like folder inheritance is wanted, and the `inherits:` mechanism
    in a folder's `index.md` is parked because Eduardo is not convinced it is the
    way.** Eduardo, 2026-08-16; no alternative is proposed anywhere.
    [Inheritance](concepts/inheritance.md), [archie init](reference/cli/init.md),
    [archie trace](reference/cli/trace.md).
24. **Structural operations on a corpus — move, rename, merge, archive — are
    archie's territory, as integral transactions that keep links valid by
    construction rather than validating and repairing afterwards.** Eduardo
    confirmed it; the blueprint records the confirmation as its one piece of
    concrete evidence for what "core of remargin" means mechanically.
    [archie and remargin](concepts/archie-and-remargin.md),
    [Changes and versions](concepts/changes-and-versions.md),
    [Move and merge with link integrity](guides/move-and-merge-with-links.md),
    [Structural operations](reference/cli/structural-operations.md).
25. **archie owns the corpus as a structured graph, with typed resources and links
    as first-class edges**, which is the shape of the answer the same capture
    draws. [Move and merge with link integrity](guides/move-and-merge-with-links.md),
    [Structural operations](reference/cli/structural-operations.md).
26. **`archie link` is almost the behavior of the `link` subcommand of Eduardo's
    `aa_tool` project, and in archie it works for any resource rather than only
    for contacts.** Eduardo, 2026-08-31; what the "almost" is waits for a feature
    discussion that has not happened. [Link resources](guides/link-resources.md),
    [archie link](reference/cli/link.md),
    [Changes and versions](concepts/changes-and-versions.md).

### remargin's side, and the one statement made outside the corpus

27. **remargin is to have a permission system in which people hold view or edit
    access to files and even to parts of markdown documents, so that a reader is
    shown only the portion they may see.** Eduardo, 2026-08-31, in the planning
    document's thread about what he had been calling "the security layer" — "this
    is what it's all about".
    [archie and remargin](concepts/archie-and-remargin.md),
    [Write through paths](guides/write-through-paths.md),
    [archie write](reference/cli/write.md),
    [Use archie from an agent](guides/use-from-an-agent.md).
28. **What exists in remargin today is a path-level permission model only**, with
    nothing below the level of a whole file.
    [archie and remargin](concepts/archie-and-remargin.md).
29. **The fate of the commands that appear only in the root design is itself an
    open question**, which is why this book marks them "root-design-only, fate
    open" rather than guessing.
    [Command-line interface](reference/cli/index.md),
    [Knowledge patterns](concepts/knowledge-patterns.md).
30. **Decided, stated outside the design corpus: archie exposes a Model Context
    Protocol server, just like remargin.** Eduardo said so in chat on 2026-08-31.
    No design document records it, which is the first entry in the gaps section
    below. [Introduction](introduction.md),
    [Model Context Protocol server](reference/mcp/index.md),
    [Model Context Protocol tools](reference/mcp/tools.md),
    [Use archie from an agent](guides/use-from-an-agent.md).

## Proposed

Twenty-seven statements in this book are Proposed: written down in the corpus,
not agreed. The first group is Eduardo's own stated direction; the rest is the
design material that predates it or was written for him to argue with.

### The direction Eduardo stated on 2026-08-25

Each of these is in his own words in the archie feature ledger with the status
`idea`, and each is a candidate goal in the requirements document, which says a
candidate becomes a goal only when he says so. He has not.

1. **A linter with shapes the user specifies for a whole markdown document or a
   sub-part of one.** [Shape language](reference/shape-language.md),
   [Shapes, schemas, and resource types](concepts/shapes-schemas-resource-types.md),
   [Define a shape](guides/define-a-shape.md).
2. **Schemas with a strict mode and a loose mode, and no stripping mode**, because
   he would not want content accepted and trimmed.
   [Shape language](reference/shape-language.md),
   [Define a shape](guides/define-a-shape.md),
   [A first session with archie](tutorial/first-session.md).
3. **The markdown document and the schema as abstractions independent of any
   file.** [Documents and schemas](concepts/documents-and-schemas.md).
4. **A transactional change model in which every change is an object**, so that a
   base plus a chain of changes replays to a byte-identical latest version and
   unapplies in reverse to the original — reachable, he said, because the content
   is linted markdown with a known shape.
   [Change objects](reference/change-objects.md),
   [Changes and versions](concepts/changes-and-versions.md),
   [Write through paths](guides/write-through-paths.md).
5. **Version tracking abstracted over pluggable storage backends**, with
   per-backend "not implemented" accepted in advance.
   [Backends and capabilities](concepts/backends-and-capabilities.md),
   [Choose a backend](guides/choose-a-backend.md),
   [Change objects](reference/change-objects.md).
6. **The boundary itself** — those five capabilities in archie, and resource
   types, the actions declared on them, and the write path in remargin. The
   blueprint records the whole placement as stated intent and not yet a design.
   [archie and remargin](concepts/archie-and-remargin.md),
   [Introduction](introduction.md).
7. **Actions declared on a remargin resource type**, a `task` with `defer`,
   `close`, and `assign` as the example, and **every write through remargin as a
   transaction.** Both are in the remargin feature ledger with the status `idea`.
   [Shapes, schemas, and resource types](concepts/shapes-schemas-resource-types.md),
   [Enforce resource types](guides/enforce-resource-types.md).

### The commands and the candidate goals behind them

8. **Every command spelling Eduardo wrote on 2026-08-30** — `ls`, `get`, `query`,
   `search`, `write`, `rm`, `link`, and `init` — because he said the list is not
   written in stone. [Command-line interface](reference/cli/index.md) and the
   page for each command.
9. **Non-markdown files, images and documents in Portable Document Format,
   retrievable as resources.** A candidate goal drawn from his `archie get`
   examples. [archie get](reference/cli/get.md),
   [Documents and schemas](concepts/documents-and-schemas.md).
10. **Parts of a document as addressable units of access**, so that remargin's
    permission system can grant view or edit on part of a markdown document. A
    candidate goal. [archie and remargin](concepts/archie-and-remargin.md).
11. **What archie would do for ProDoctivity** — linting, schema enforcement, the
    security layer, and referential integrity — recorded as a candidate list
    because Eduardo prefaced it with "probably by".
    [archie and remargin](concepts/archie-and-remargin.md).

### The root design's own layer

12. **The catalog of twelve validation rules**, each with its level, its message,
    and whether a repair can address it.
    [Validation rules](reference/validation-rules.md),
    [archie check](reference/cli/check.md).
13. **The three knowledge-pattern rules**, whose levels come from configuration
    rather than being fixed. [Validation rules](reference/validation-rules.md).
14. **The whole configuration file**: the corpus-root table with its ignore list,
    the per-rule switches, the fixed type list, the cascading properties, the
    archive keys, the inbox keys, the asset extensions, the language-model keys,
    the output keys, and the knowledge-pattern block.
    [Configuration](reference/config.md).
15. **The precedence order of configuration sources and the five environment
    variables.** [Configuration](reference/config.md),
    [Command-line interface](reference/cli/index.md).
16. **The six exit codes, and the three requirements for an error message with
    the illustration that meets them.** [Exit codes](reference/exit-codes.md).
17. **The three output formats of a validation run** — text, JavaScript Object
    Notation, and markdown. [Output formats](reference/output-formats.md).
18. **The five design principles**, including that archie is friendly to
    continuous integration through exit codes and machine-readable output.
    [Run archie in continuous integration](guides/run-in-ci.md),
    [Exit codes](reference/exit-codes.md).
19. **Rules declared in the configuration**: a custom-rule table, a hook table
    naming an external command, and an embedded scripting language marked "if
    needed". [Configuration](reference/config.md).
20. **The knowledge patterns themselves**: observations with eight categories,
    relations with nine types and their inverses, the placement convention, and
    the export document's shape.
    [Knowledge patterns](concepts/knowledge-patterns.md),
    [archie export](reference/cli/export.md).
21. **Basic Memory as the source of the pattern idea**, and the comparison the
    root design draws against it.
    [Knowledge patterns](concepts/knowledge-patterns.md).
22. **The whole inheritance mechanism**: the four rules, accumulation down the
    tree, the absence of an override, the `allows:` block, the three rules that
    enforce it, the command that prints the chain, and the philosophy it serves —
    folder-first organization, every file a resource, one global inbox with
    per-area archives, archiving triggered only by a property, and the inbox as
    the one place the rules relax. [Inheritance](concepts/inheritance.md),
    [archie trace](reference/cli/trace.md).

### What is written down but explicitly undesigned

23. **Structural operations in every detail**: which reference spellings get
    rewritten and which are preserved, how the operation reports what it rewrote,
    and what a dry run does.
    [Structural operations](reference/cli/structural-operations.md),
    [Move and merge with link integrity](guides/move-and-merge-with-links.md).
24. **The merge-move**: union entry by entry into an existing destination, a
    collision policy of failing, skipping, or overwriting with failing as the
    default, cleanup of an emptied source folder, inbound-link rewriting by
    composition, and atomicity.
    [Structural operations](reference/cli/structural-operations.md),
    [Changes and versions](concepts/changes-and-versions.md).
25. **The grammar strawman in full**: one expression language with three entry
    points, the document model it walks, the three clause forms, chaining and
    projection, the five mutation verbs, search as a query with a default body
    stage, and one invocation as one transaction.
    [Query grammar](reference/query-grammar.md),
    [archie query](reference/cli/query.md),
    [archie search](reference/cli/search.md).
26. **The reading of `# ##["Specs"][2]`** as a heading level, then a heading
    title, then an index among children — marked in the corpus as an assumption
    about Eduardo's example rather than a statement of his.
    [archie write](reference/cli/write.md),
    [Write through paths](guides/write-through-paths.md).
27. **Everything about the Model Context Protocol server** beyond the fact that
    there is one: the command that starts it, how it resolves its configuration,
    one tool per command, columnar payloads with an exact total, and a non-zero
    exit becoming a protocol error. Every one of those is a reading of "just like
    remargin" against remargin's own server.
    [Model Context Protocol server](reference/mcp/index.md),
    [Model Context Protocol tools](reference/mcp/tools.md).

One reading sits between the two lists and is marked Proposed on its pages: the
archie feature ledger reads the `link` and `unlink` pair of `aa_tool` as the
change-object model applied to one kind of edit. The behavior of that program is
readable in its source; the reading of it as an instance of the change model is
as unsettled as the model.
[Change objects](reference/change-objects.md),
[Link resources](guides/link-resources.md).

## Suggestions

This is the list Eduardo reviews. Every entry is an addition this book makes that
the design corpus does not contain, rendered on its page as a blockquote that
opens with **Suggestion (documentation author):** and that names the decision it
asks him for. Twenty-five of them, in the order the book presents them.

1. **Whether a first session has a prescribed order**, and whether it is the
   eight steps this book chose — nothing in the corpus describes a session or a
   starting point. [A first session with archie](tutorial/first-session.md).
2. **Whether reading a document's history is part of archie's surface at all**,
   and if so what a reader asks for: a whole chain, one change object, or a
   difference between two versions. No command in the corpus reads history.
   [A first session with archie](tutorial/first-session.md).
3. **Whether the strict or loose mode belongs to the shape or to the check**,
   which decides whether choosing a mode is a step in declaring a shape or an
   option of the command. [Define a shape](guides/define-a-shape.md).
4. **How remargin's type definitions reach archie's linter** — passed at call
   time, kept in files archie reads, or held in archie's configuration.
   [Enforce resource types](guides/enforce-resource-types.md).
5. **Whether archie documents and supports an adoption path** for a corpus that
   has never been checked, including a way to record which existing findings are
   accepted. [Lint a vault](guides/lint-a-vault.md).
6. **How a build machine obtains archie**, which is the gap that makes the
   starter workflow file on that page a sketch, and whether archie ships a
   workflow, a pre-commit hook, and shell completions at all.
   [Run archie in continuous integration](guides/run-in-ci.md).
7. **Whether a write is previewed before it commits**, and if so whether the
   preview is an option on the command or a separate call returning the change
   object. [Write through paths](guides/write-through-paths.md).
8. **The tool set, the preview step, and the boundary with remargin's own
   server** — the three decisions an agent-facing surface needs before it can be
   named. [Use archie from an agent](guides/use-from-an-agent.md).
9. **Whether "shape" and "schema" name one object or two.** The corpus uses both
   words, from the same conversation, without ever relating them.
   [Shapes, schemas, and resource types](concepts/shapes-schemas-resource-types.md).
10. **A capability matrix for backends, and whether a backend declares its
    capabilities up front** or "not implemented" is only ever an error from an
    attempted call.
    [Backends and capabilities](concepts/backends-and-capabilities.md).
11. **Whether the system ends with one Model Context Protocol server or two**,
    given that archie is to have one and the fate of remargin's is undecided.
    [archie and remargin](concepts/archie-and-remargin.md).
12. **A notation for writing a shape down**, offered as a block in the
    configuration file to react to, together with the three decisions behind it:
    the notation, the binding, and the vocabulary for body structure.
    [Shape language](reference/shape-language.md).
13. **A change object's format, its identity, and its storage** — a patch or an
    operation over the document tree, a content hash or a sequence number, and
    whether the chain is the backend's responsibility or archie's.
    [Change objects](reference/change-objects.md).
14. **Whether one exit-code table serves every command or each verb defines its
    own**, and whether "found nothing" is a distinct code.
    [Exit codes](reference/exit-codes.md).
15. **Whether the four global options remain the program-wide contract** that
    every command honors, including the commands Eduardo named.
    [Command-line interface](reference/cli/index.md).
16. **What a folder retrieval, a markdown retrieval, and a binary retrieval each
    return.** [archie get](reference/cli/get.md).
17. **What a row of a listing carries, whether it recurses, whether it lists
    non-markdown files, and whether the ignore list applies.**
    [archie ls](reference/cli/ls.md).
18. **Whether a removal is a change object that can be unapplied**, and what
    happens to inbound links to what was removed.
    [archie rm](reference/cli/rm.md).
19. **Command spellings for move, rename, merge, and archive** — one verb each, a
    single verb with a mode, or no command at all because they are engine
    operations remargin calls.
    [Structural operations](reference/cli/structural-operations.md).
20. **The spelling and shape of the command that starts the server** — a
    subcommand, an option, or a second binary.
    [Model Context Protocol server](reference/mcp/index.md).
21. **How the configuration precedence order reads for a server that keeps
    running**: whether tool-call parameters stand where command-line options
    stand, whether the file is re-read when it changes, and whether one server
    may serve more than one corpus.
    [Model Context Protocol server](reference/mcp/index.md).
22. **Which exit codes become protocol errors** and which are results the caller
    reads. [Model Context Protocol server](reference/mcp/index.md).
23. **Whether the structural operations get tools before they get command
    spellings, or after.** [Model Context Protocol tools](reference/mcp/tools.md).
24. **What becomes of the two commands that ask a person questions** on a tool
    surface — drop the interactive mode, split the operation in two, or give them
    no tool. [Model Context Protocol tools](reference/mcp/tools.md).
25. **A default result shape for every tool whose command output the corpus does
    not specify**, decided once rather than per tool.
    [Model Context Protocol tools](reference/mcp/tools.md).

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

Every place this book had to say something the corpus does not say, or had to
leave a page thin because the corpus is silent. Seven of them are contradictions
or silences that earlier passes of this documentation raised separately, and each
is recorded here because recording it is all a documentation page can do about
it: none can be closed by writing, only by Eduardo deciding.

### Contradictions inside the corpus

1. **Whether a missing frontmatter block can be repaired without a person.** The
   root design's repair command lists four repairs and creating a frontmatter
   block is not among them, while the sample machine-readable output in the same
   document shows a `frontmatter-required` finding carrying a repairable flag set
   to true. Nothing resolves it, and the whole catalog is already under the
   deferral note, so the resolution may be that the question retires with it.
   Affects [Validation rules](reference/validation-rules.md),
   [archie fix](reference/cli/fix.md),
   [Output formats](reference/output-formats.md),
   [Run archie in continuous integration](guides/run-in-ci.md).
2. **Whether archie builds a knowledge graph.** The root design's tools-comparison
   table gives archie "an inheritance tree only" against Basic Memory's semantic
   knowledge graph, while the export format in the same document emits a `graph`
   object of nodes and typed edges built from the relations in document bodies.
   Eduardo has stated no position on that whole section. Affects
   [Knowledge patterns](concepts/knowledge-patterns.md),
   [archie export](reference/cli/export.md).
3. **The configuration still calls its corpus a vault and ignores Obsidian's own
   folder.** Eduardo stated on 2026-08-30 that nothing about archie will require
   Obsidian, and the root design's opening sentence was rewritten to match, but
   the configuration section was not revisited: its first table is named `vault`,
   its environment variable is described as the vault root path, and the first
   entry of the default ignore list is Obsidian's folder. Nothing says whether the
   table is renamed, whether the folder stays as a convenience default, or whether
   the wording is simply stale. Affects [Configuration](reference/config.md),
   [Command-line interface](reference/cli/index.md).

### Silences the corpus has not filled

4. **How a shape relates to a schema, and how either relates to a resource type.**
   Both words come from the same conversation on 2026-08-25 and no document
   relates them: one object under two names, a schema as a named collection of
   shapes plus a mode, or a shape as the part of a schema that governs a sub-part
   are all available and none is ruled out. The relation to a remargin resource
   type is equally unstated. Affects
   [Shapes, schemas, and resource types](concepts/shapes-schemas-resource-types.md),
   [Shape language](reference/shape-language.md),
   [Define a shape](guides/define-a-shape.md).
5. **Backend-agnostic version tracking has one paragraph and no interface.** The
   corpus names no operation a backend provides, does not say whether a backend
   stores documents or change objects or both, does not say whether "not
   implemented" is discovered by asking or by attempting, gives no way to choose
   or configure a backend, and marks no part of the interface as required of
   every backend. Affects
   [Backends and capabilities](concepts/backends-and-capabilities.md),
   [Choose a backend](guides/choose-a-backend.md),
   [Change objects](reference/change-objects.md).
6. **How a folder rule names resource types now that resource types are
   remargin's.** A folder's `index.md` declares an `allows:` block listing the
   types permitted in the folder, and the root design turns that into the
   `allowed-types` rule — both written before types moved to remargin on
   2026-08-30. Nothing says whether the block survives, becomes a shape, or moves
   to remargin with the types. A second silence rides with it: inheritance is
   bound to folders and to a file named `index.md` while shapes are bound to
   documents and their parts, and nothing relates the two, so it is unstated
   whether inheritance survives the schema-driven rework as a folder mechanism at
   all. Affects [Inheritance](concepts/inheritance.md),
   [Enforce resource types](guides/enforce-resource-types.md).
7. **Whether archie and remargin expose one Model Context Protocol server or
   two.** Eduardo said in chat on 2026-08-31 that archie exposes one "just like
   remargin", and the vision blueprint lists remargin's own server among the
   subsystems whose ownership is undecided. Nothing says whether the end state is
   two servers with their own tools or one server whose tools come from both
   layers. Affects [archie and remargin](concepts/archie-and-remargin.md),
   [Model Context Protocol server](reference/mcp/index.md),
   [Model Context Protocol tools](reference/mcp/tools.md).

### Where this book had to stand on something outside the corpus

8. **The Model Context Protocol server itself is a chat statement.** Eduardo said
   on 2026-08-31 that archie exposes one; no document in the design corpus
   records it — not the root design, whose module layout has no server module and
   whose command list names no command that starts one, and not the planning
   document or the blueprint. Two reference pages and one guide rest on that
   sentence, and every page that mentions it cites the statement with its date.
   [Model Context Protocol server](reference/mcp/index.md),
   [Model Context Protocol tools](reference/mcp/tools.md),
   [Use archie from an agent](guides/use-from-an-agent.md).
9. **The validation command has two names and nothing reconciles them.** The root
   design calls it `check` and writes every option under that name; Eduardo's
   list has `lint` in the same position and never says `check`. This book keeps
   both on record and chooses neither. [archie check](reference/cli/check.md),
   [Lint a vault](guides/lint-a-vault.md),
   [Validation rules](reference/validation-rules.md).
10. **`archie query` has two shapes and nothing reconciles them either** — the
    expression and the path as positional arguments in 2026-08-16, the expression
    as a named option with no path in 2026-08-30. The difference does not survive
    into a tool call, where every parameter is named.
    [archie query](reference/cli/query.md),
    [Model Context Protocol tools](reference/mcp/tools.md).
11. **The structural operations have no command spelling at all**, which is why
    this book documents them on one page rather than one page per verb, and shows
    no command line for them.
    [Structural operations](reference/cli/structural-operations.md),
    [Move and merge with link integrity](guides/move-and-merge-with-links.md).
12. **Output is written down for four commands only** — the validation run, the
    inheritance trace, the inbox proposal, and the export document — so every
    other page in this book shows a command line with no result under it. Exit
    codes are written for the validation run alone, so no other command's return
    to the shell is documented anywhere.
    [Output formats](reference/output-formats.md),
    [Exit codes](reference/exit-codes.md).
13. **Nothing describes how archie is obtained.** No published package, no release
    artifact, no container image, no installation instruction of any kind — which
    is why the workflow file in the continuous-integration guide has a placeholder
    where its install step belongs, and why that whole block is a Suggestion. The
    root design's implementation plan names an example workflow, a pre-commit
    hook, and shell completions as work to be done, and the corpus contains none
    of the three. [Run archie in continuous integration](guides/run-in-ci.md).
14. **Two details inside the corpus-specified output are unexplained.** The inbox
    illustration proposes a relation of type `attendees`, which is not among the
    nine relation types the same document lists, so the screen shows a proposal
    the relation-type rule would report as unknown. And the export document writes
    a relation's targets as the names found in wiki-links while writing the
    graph's nodes and edges as file names, so it performs a resolution from a name
    to a file that no document describes.
    [archie inbox](reference/cli/inbox.md),
    [archie export](reference/cli/export.md),
    [Knowledge patterns](concepts/knowledge-patterns.md).
15. **The sample output still carries literal version strings.** The root design's
    text and machine-readable samples print a version banner, and the corpus
    records that those can become neutral placeholders on Eduardo's word. He has
    not given it, so this book reproduces the samples as written and omits the
    banner line where it quotes them.
    [Output formats](reference/output-formats.md),
    [archie check](reference/cli/check.md).
16. **The performance goals for the parser do not exist yet.** Eduardo requires
    the parser to be a good one and will state the goals; until he does, no page
    in this book makes any claim about how fast anything is, and the requirements
    document holds a placeholder where the criteria would go.
    [Documents and schemas](concepts/documents-and-schemas.md),
    [Query grammar](reference/query-grammar.md).

## Where to start reading

- [Introduction](introduction.md) — what archie is, and how to read the markers.
- [A first session with archie](tutorial/first-session.md) — the eight steps, and
  the three of them that end without a command line.
- [Command-line interface](reference/cli/index.md) — the command inventory with
  each command's status.
