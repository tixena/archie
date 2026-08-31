# A first session with archie

**Status:** Mixed — see markers.

This is one pass through archie from an empty corpus to a change: set the corpus
up, describe what its documents must look like, check them, read the report,
select documents, search their text, write into one of them, link the mentions
inside them, and see what the change amounts to. Each step below says how settled
it is, and three of the eight steps end without a command line because the design
corpus does not yet contain one.

## Before you start

**Decided.** No archie binary exists. archie is in its design phase and no code
has been written: the vision blueprint recorded on 2026-08-16 that the repository
holds only its task-tracking configuration, and the grammar document repeated it
on 2026-08-31. Nothing below has been run. Every console block is a command line
the design corpus contains, not a transcript.

The command lines here are Eduardo Burgos's own, quoted as he wrote them. His
list of 2026-08-30 on the planning document already uses `folder/` and
`folder/file.md` as its placeholders, so this session keeps those names rather
than inventing a worked example around them. The one exception is the validation
report in step 3, which is the root design document's own example and names that
document's own paths.

**Proposed.** Every command Eduardo named is Proposed, even where something about
its behavior is Decided. He wrote the list on 2026-08-30 as the things he would
actually run, and stated on the same document that it is not written in stone —
the verbs and the syntax can change and are not a hard requirement. That
statement is itself recorded as a settled point on the grammar document.

**Output.** The design corpus specifies program output in exactly one place: the
root design document's output-format section, which describes what a validation
run prints. So step 3 shows output and no other step does.

> **Suggestion (documentation author):** the order of the eight steps below is
> this book's arrangement and not the design's. No document in the corpus
> describes a session, a sequence, or a starting point for someone holding
> archie for the first time — the closest thing to one is Eduardo's list of
> commands, which is a list and not an order. In particular, nothing states that
> a corpus has to be initialized before any other command will run on it, so
> putting `archie init` first is this page's choice rather than a requirement.
> Eduardo would have to decide whether a first session has a prescribed order,
> and if so whether it is this one.

## Step 1: set the corpus up

```console
$ archie init .
```

**Proposed.** This is Eduardo's line from 2026-08-30. Unlike the repair,
classification, inheritance-tracing, export, and import commands, `init` is in
his list, so it does not fall under the vision blueprint's open question about
what becomes of archie's features for maintaining a personal corpus. Its option
set, however, comes only from the root design document, which spells the command
`archie init [PATH]` and gives it three options: `--config` creates the
configuration file if it is missing, `--indexes` creates the folder index files
that are missing, and `--all` does both.

**Root design.** The first of those two jobs writes `archie.toml` at the root of
the repository — the file whose every key is listed on
[Configuration](../reference/config.md). The second creates an `index.md` in
every folder that has none, carrying the properties that folder inherits from the
folders above it.

**Parked.** The mechanism the second job serves is under reconsideration. Eduardo
said on 2026-08-16 that he wants something like property inheritance from folder
index files but is not convinced that the `inherits:` block is the way it should
work, and asked to discuss other aspects first. Nothing has replaced it. See
[Inheritance](../concepts/inheritance.md).

**Not in the corpus.** What `archie init` does when none of the three options is
given, what the file it writes contains, whether an existing configuration file
is left alone or refreshed, and what the path defaults to when it is omitted. The
design specifies no output for the command, so none is shown here. See
[archie init](../reference/cli/init.md).

## Step 2: describe what the documents must look like

This step has no command line, and it is the step that everything after it leans
on.

**Decided.** archie has a linter, and the shapes it checks against are written by
the user. Eduardo stated it on 2026-08-25 and the archie feature ledger records
it in his words: "archie must have a linter. I can specify the *shape* of any
markdown (or sub-markdown part)." A shape therefore governs either a whole
markdown document or one part of a document.

**Decided.** A schema has two modes and only two. Strict rejects content that
falls outside the declared shape; loose lets that content through. There is no
mode that accepts a document and trims it — in his words, from the same
statement, "I wouldn't want to accept content and trim it."

**Not in the corpus.** How a shape is written down, and how it reaches the
documents it governs. No document in the corpus contains a shape written out,
there is no example of a shape declaration anywhere, and the absence of a schema
language and of a way to attach schemas to folders and to files has been recorded
as an open item on the root design document since 2026-07-27. So this session
cannot show you a shape, and the steps that follow check documents against a
fixed rule catalog rather than against anything you wrote.

The reference page [Shape language](../reference/shape-language.md) records the
same absence at length and offers one sketch of a notation as a Suggestion. The
guide [Define a shape](../guides/define-a-shape.md) walks the four decisions that
declaring a shape needs, two of which have answers and two of which do not.

## Step 3: check the corpus and read the report

```console
$ archie lint folder/
```

**Proposed.** That is Eduardo's spelling, from his 2026-08-30 list. The root
design document calls the same command `check` and writes every option for that
name; no document reconciles the two names, and this book keeps both on record.
The `check` spelling carries the further qualification that it appears in the
root design and not in Eduardo's list, which places it among the commands whose
fate the vision blueprint holds open.

**Root design.** A run walks the corpus root named in the configuration file, or
the path it is given, skips the folders in the ignore list, applies the enabled
rules, and reports every finding at one of three levels. It never modifies a
file: reporting and repairing are separate acts, and only the repair run writes.

This is the report the root design specifies, under its own spelling of the
command, with the run's opening banner line omitted because this book carries no
release framing:

```console
$ archie check
Scanning src/...
Found 127 markdown files, 45 assets

ERRORS (3):
  ✗ src/notes/random.md
    Missing frontmatter

  ✗ src/tixena/new/
    Missing index.md

  ✗ src/tixena/projects/old.md
    archived=true but not in archive/

WARNINGS (2):
  ⚠ src/prodoctivity/notes/planning.md
    Contains 5 embedded task items

  ⚠ src/tixena/assets/screenshot.png
    Not referenced by any file

INFO (4):
  ○ src/inbox/note1.md - Needs processing
  ○ src/inbox/note2.md - Needs processing
  ○ src/inbox/note3.md - Needs processing
  ○ src/inbox/note4.md - Needs processing

─────────────────────────────────────
Summary: 3 errors, 2 warnings, 4 info
         122 files OK
```

Read it in three parts. **Errors** are documents that break a rule the design
treats as a violation — no frontmatter, frontmatter that does not parse, a
missing or unrecognized `type`, a missing `created` date, a missing inherited
property, a folder with no index file, a file whose type its folder does not
allow, and a file marked as archived that sits outside an archive folder.
**Warnings** are the two findings that ask for a judgment: an asset that no
markdown file references, and a note holding several independent checkbox items.
**Information** is the inbox: files that still have no frontmatter and are
waiting to be classified.

Four options shape the run, all of them written for the `check` spelling only.
`--format` selects plain text, JavaScript Object Notation (JSON), or markdown;
`--fail-on` takes `error`, `warning`, or `any` and decides the process exit
status and nothing else; `--rule` restricts the run to named rules and may be
given more than once; `--ignore` adds patterns to skip on top of the
configuration file's list. The other two formats of this same report are shown in
full on [Output formats](../reference/output-formats.md), and the exit-code table
is on [Exit codes](../reference/exit-codes.md).

**Deferred.** The rules this run applies are scheduled to be replaced. Eduardo
recorded the deferral on 2026-08-16 at the top of the root design's validation
section: the catalog encodes an earlier, hard-coded model — frontmatter required
everywhere with the inbox as the lone exception, `type` and `created` as fixed
requirements, and a fixed list of resource types — and under the schema-driven
direction each rule becomes a consequence of what the schemas say. So the report
above is what archie's design specifies today, and not what it is heading for.
Every rule with its message is on
[Validation rules](../reference/validation-rules.md), and the guide
[Lint a vault](../guides/lint-a-vault.md) covers the run in full.

## Step 4: select documents by their frontmatter

```console
$ archie query 'tags contains "rule"' .grit/patterns/
$ archie query --exp='type=issue'
```

**Proposed spelling.** Both lines are Eduardo's, and they are the whole of what
the corpus writes down: the first from 2026-08-16, with the expression and the
path as positional arguments, and the second from 2026-08-30, with the expression
as the value of a named option and no path at all. No document reconciles them,
and the grammar document lists the spelling of the language — its operators, its
quoting, its escaping, and how an expression is written as a command-line
argument — among the things nobody has settled. The path in the first line is the
folder Eduardo wanted to query, kept as he wrote it.

**Decided.** This is the first thing archie is to do. He confirmed on 2026-08-16,
on the frontmatter and architecture discussion, that
`archie query 'tags contains "rule"' .grit/patterns/` is the first thing to
build, and that starting is on his signal because he wanted to write other things
down first. He has not given the signal.

**Decided.** The direction of the language. In his words, stated on 2026-08-16
and restated on 2026-08-30: "I'd like something like `jq` because it is extremely
powerful and also I'd like to be able to chain queries and be able to project
almost anything." `jq` is a command-line processor for JSON in which small
filters join into pipelines. So expressions chain, and a query can name what it
emits instead of always emitting whole documents.

**Depends on Proposed grammar.** Beyond those two lines, every clause form is a
strawman put in the grammar document for Eduardo to argue with — the frontmatter
comparison, the membership test, the stages joined by a vertical bar, and the
projection. Nothing in it is settled until it is rewritten into that document's
decisions section, so this step shows no third query. See
[Query grammar](../reference/query-grammar.md).

**Not in the corpus.** What a query prints. The design specifies output for the
validation run and for nothing else, so no result is shown here, and nothing
states what the command returns to the shell when nothing matched. See
[archie query](../reference/cli/query.md).

## Step 5: search the text of the documents

```console
$ archie search folder/ -gi "[0-9]?[a-z]*"
```

**Proposed spelling.** Eduardo's own line from 2026-08-30, and the only spelling
of `archie search` anywhere in the corpus.

**Decided.** What a search covers. He stated the scope on 2026-08-31, in reply to
a question on the planning document, and it is recorded as a settled point on the
grammar document: search "should be able to run on document bodies and on
frontmatter if you specify it (and both if you give clauses for both)." He named
that requirement as one of his reasons for archie writing its own markdown
parser, because matching frontmatter and body separately means knowing where one
ends and the other begins.

**Not in the corpus.** How you would spell that. Nothing states the syntax of a
frontmatter clause or of a body clause, even though giving both is settled
behavior, so this step shows no second command line. The two short flags in the
line above are undefined as well: an agent asked on 2026-08-31 whether they are
the grep-style flags for a global, case-insensitive match, and the question was
dropped when Eduardo answered the scope question instead and said the syntax is
not a hard requirement. Which dialect of regular expression the pattern is
written in is not stated either. See
[archie search](../reference/cli/search.md).

## Step 6: write into part of one document

```console
$ archie write folder/file.md --path='# ##["Specs"][2]' --append --from-body='path-to-file.txt'
```

**Proposed spelling.** Eduardo's own line from 2026-08-30, and the only spelling
of `archie write` in the corpus. It names one document, addresses a part of it
with `--path`, appends rather than replaces, and takes the content to append from
the file named by `--from-body`.

**Depends on Proposed grammar, and on less than that.** The expression
`# ##["Specs"][2]` is not defined anywhere. Asked on 2026-08-31 to say what each
piece of it selects, Eduardo answered: "We need to discuss extensively about this
grammar. So, as you may understand I can't reply it in this comment. More
importantly I don't know all the answers. I only have a vague idea of what I
want." He asked for a document dedicated to the grammar, which is why the grammar
document exists. That document's strawman reads the expression literally, as a
heading level, then a heading title, then an index among the children — selecting
the second block under the level-two heading titled "Specs" — and marks the
reading as an assumption about his example rather than a statement of his.

**Decided.** Appending is the one mutation verb on record. It is in the command
line above, and it is also the verb in his description of a bulk change: append a
section carrying the current date to each of the elements a query returns. The
four other verbs in the strawman — prepending, setting, removing, and replacing —
come from the strawman and not from him.

**Decided.** Reaching into a document is the direction rather than an accident of
one example. Shapes govern sub-parts of documents, modification driven by a query
reaches inside a document, and Eduardo stated on 2026-08-31 that remargin's
permission system gives people access to files "and they can even have access to
parts of markdowns". Whatever a path expression can address is what a shape can
govern and what such a permission can be granted on.

**Not in the corpus.** Whether the content may be given inline or read from
standard input instead of from a file, what happens when the path addresses
nothing, what happens when the document does not have the structure the path
assumes, whether a write can be previewed before it commits, and what the command
prints. See [archie write](../reference/cli/write.md) and the guide
[Write through paths](../guides/write-through-paths.md).

## Step 7: turn mentions into links

```console
$ archie link folder/
```

**Proposed spelling.** Eduardo's own line from 2026-08-30, and the only spelling
of the command in the corpus. No option of `archie link` is written down.

**Decided.** What it does, by reference. Asked on 2026-08-31 what
`archie link folder/` does, he answered: "It is ALMOST the same behavior as the
`link` command in my `aa_tool` project", and added, "That part is important to
me." That program wraps every collision-free mention of a name or alias, found in
the markdown files in scope, in a markdown link whose target is a relative path
to the record for that name. It preserves the matched text exactly, matches the
longest surface first and only on word boundaries, refuses to link a string that
two records both claim, and never rewrites frontmatter, fenced code blocks,
inline code spans, existing links and wiki-links, or raw web addresses.

**Decided.** The generalization. In his words, the same day: "In archie it
doesn't only works for contacts. It can work for whatever." So archie links
mentions of a resource to that resource, whatever kind of resource it is. What
counts as a resource is declared on remargin's side — Eduardo settled on
2026-08-30 that resource types are business logic and belong to remargin, with
archie holding only the means to enforce them.

**Not in the corpus.** What the "almost" is. He deferred the rest to a feature
discussion that has not happened, so every difference beyond the widening from
people to any resource is unstated: which frontmatter fields of a resource supply
its surface strings, how a target is chosen when the resource is a folder, what a
report of the run looks like, and whether archie has an inverse command that
strips the links this one added. The archie feature ledger records the entry with
the status `idea`. See [archie link](../reference/cli/link.md) and the guide
[Link resources](../guides/link-resources.md).

## Step 8: see what the change amounts to

The session has now written into documents twice — once through a path and once
by linking. This step has no command line either.

**Decided, with its own qualification.** Every change produces an object that
represents a change to a resource, and the model is the one git uses. Eduardo
stated it on 2026-08-25: given a base resource and a chain of changes applied in
sequence you obtain a byte-identical latest version, and unapplying the same
chain in reverse returns the original. The archie feature ledger records the
entry with the status `idea` and the requirements document lists it among the
candidate goals he has not yet accepted as goals, so what is settled is that he
stated it and that it falls on archie's side of the boundary he drew on
2026-08-25 between archie and remargin.

**Decided.** The reason the guarantee is reachable is stated with it: the content
is linted markdown with a known shape. A change to arbitrary text can only be
recorded as a difference between two texts; a change to a document whose
structure is known can be recorded in terms of that structure. That is why step 2
sits where it does, and why the linter is underneath everything the later steps
did.

**Decided, as an observation.** The link run in step 7 is that model in
miniature. It is additive, so stripping every link it injected reproduces the
file byte for byte; it is idempotent, because text already inside a link is a
region it skips; and the reference implementation's inverse strips only a link
that the run would have produced. The archie feature ledger records that reading.

**Not in the corpus.** What a change object contains, how it is serialized, how
one is named, where the chain is kept, whether one query-driven change across
many documents is one object or one per document, and what archie does when a
replay is not byte-identical because a document was edited outside archie. See
[Change objects](../reference/change-objects.md) and
[Changes and versions](../concepts/changes-and-versions.md).

> **Suggestion (documentation author):** no command in archie's design reads a
> document's history. The command inventory holds nothing that shows the chain of
> changes behind a document, the change object a particular write produced, or
> the difference between two versions — the closest is the command that prints
> the chain of properties a file inherits, which is a different subject. So a
> session can make changes and then has no way to look at them, which is why this
> step ends without a console block. Eduardo would have to decide whether reading
> history is part of archie's surface at all or belongs entirely to remargin and
> to the storage backend underneath, and, if it is archie's, what a reader asks
> for: a document's whole chain, one change object, or the difference between two
> named versions.

## Where to go next

Each step above has a guide that takes it further.

- [Define a shape](../guides/define-a-shape.md) — the four decisions that
  declaring a shape needs, and which two of them have answers.
- [Lint a vault](../guides/lint-a-vault.md) — the validation run in full: the
  report, narrowing it, choosing when it fails, and repairing what can be
  repaired.
- [Query and search](../guides/query-and-search.md) — choosing between the two
  reading verbs, and what is settled about each.
- [Write through paths](../guides/write-through-paths.md) — addressing part of a
  document, and what the write commits.
- [Link resources](../guides/link-resources.md) — the reference implementation
  read step by step, and what archie inherits from it.
- [Enforce resource types](../guides/enforce-resource-types.md) — the same
  machinery seen from remargin's side of the boundary.
- [Move and merge with link integrity](../guides/move-and-merge-with-links.md) —
  moving a folder without breaking the references into it.
- [Choose a backend](../guides/choose-a-backend.md) — where a corpus and its
  versions are stored.
- [Use archie from an agent](../guides/use-from-an-agent.md) — the Model Context
  Protocol (MCP) server, which Eduardo said in chat on 2026-08-31 that archie
  exposes, just like remargin, and which no design document records yet.
- [Run archie in continuous integration](../guides/run-in-ci.md) — the same check
  as a build step.

For how settled the design is as a whole — what is decided, what is proposed,
which additions this book makes, and which questions the corpus leaves open — see
[Design status](../design-status.md).
