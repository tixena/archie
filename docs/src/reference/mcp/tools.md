# Model Context Protocol tools

**Status:** Mixed — see markers.

This page lists the tools archie's Model Context Protocol (MCP) server would
expose, one entry per tool. Nothing on it is settled. The design corpus contains
no tool list, no parameter name, and no result shape for archie: it does not
record that the server exists at all. What it records is the command line, and
Eduardo Burgos's statement in chat on 2026-08-31 that archie exposes a server
"just like remargin" — a program whose tools are its commands. So every entry
below is a command from the [command-line interface](../cli/index.md) seen as a
tool, marked **Proposed**, or an addition of this book's, marked as a
**Suggestion**. Where an entry below marks something **Decided**, that decision
is about the command the tool mirrors and never about the tool, of which nothing
is decided. Read the [server page](index.md) first; it holds where the statement
comes from, what remargin's conventions are, and what the corpus leaves silent.

## How to read an entry

Each entry gives the tool's name, the command it mirrors, what it is for, its
parameters, what it returns, and its status.

**A tool is never more settled than the command it mirrors.** Eduardo stated on
2026-08-30 that his command list is not written in stone — the verbs and the
syntax can change and are not a hard requirement — and that statement is
recorded as a settled point on the grammar document. Six further commands come
only from the root design document and fall under the vision blueprint's open
question about what becomes of archie's features for maintaining a personal
corpus; their entries say so. Each entry's status is the status of its command,
carried over.

**Where a parameter comes from is always stated.** A parameter written into a
command line that Eduardo or the root design wrote is marked as such. A
parameter that comes only from the shape of remargin's corresponding tool is
marked as such, and it is an inference, not a statement of archie's design.
Where neither has it, the entry says the corpus is silent rather than filling
the gap.

**Names follow remargin's convention.** Tool names are the command names, short
and without a prefix, because the server scopes them — settled for remargin in a
thread of 2026-04-12 in which Eduardo asked for short names on the ground that
calling a Model Context Protocol server costs the client tokens. A parameter
takes its option's name with the dashes replaced by underscores, the way
remargin spells `dry_run` and `ignore_case`. Types are the ordinary ones a tool
schema carries: a string, a boolean, a number, an array of strings.

## The tools

| Tool | Mirrors | Status |
|---|---|---|
| [`ls`](#ls) | [`archie ls`](../cli/ls.md) | Proposed |
| [`get`](#get) | [`archie get`](../cli/get.md) | Proposed |
| [`query`](#query) | [`archie query`](../cli/query.md) | Proposed, with a decided starting point |
| [`search`](#search) | [`archie search`](../cli/search.md) | Proposed, with a decided scope |
| [`write`](#write) | [`archie write`](../cli/write.md) | Proposed |
| [`rm`](#rm) | [`archie rm`](../cli/rm.md) | Proposed |
| [`link`](#link) | [`archie link`](../cli/link.md) | Proposed |
| [`lint`](#lint) | [`archie check`](../cli/check.md), which Eduardo's list spells `archie lint` | Root-design-only, fate open |
| [`init`](#init) | [`archie init`](../cli/init.md) | Proposed |
| [`fix`](#fix) | [`archie fix`](../cli/fix.md) | Root-design-only, fate open |
| [`inbox`](#inbox) | [`archie inbox`](../cli/inbox.md) | Root-design-only, fate open |
| [`trace`](#trace) | [`archie trace`](../cli/trace.md) | Root-design-only, fate open |
| [`export`](#export) | [`archie export`](../cli/export.md) | Root-design-only, fate open |
| [`import`](#import) | [`archie import`](../cli/import.md) | Root-design-only, fate open |

One entry of the command inventory has no tool, and two other things this book
describes cannot have one yet. All three are covered under
[commands with no tool](#commands-with-no-tool) at the end.

## ls

**Mirrors** [`archie ls`](../cli/ls.md). **Proposed.**

Lists the contents of a corpus. The corpus names the command and describes no
behavior for it: Eduardo wrote the bare verb `archie ls`, with no argument, on
2026-08-30.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string | Not from archie's corpus. remargin's `ls` takes a directory path that defaults to the directory the server was started in; a tool with no parameter at all could only ever list one place. |

**Returns.** Not specified. Nothing states what a row of a listing carries, or
whether a listing recurses, so nothing states what the tool returns. The
`archie ls` page records the four questions Eduardo would have to answer before
a listing can be described at all.

## get

**Mirrors** [`archie get`](../cli/get.md). **Proposed.**

Retrieves a resource by its path. Eduardo's four example command lines of
2026-08-30 reach for a folder, a markdown file, an image, and a document in
Portable Document Format.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string, required | The argument in all four of Eduardo's command lines. |

**Returns.** Not specified. The corpus does not say what a folder retrieval
returns as against a file retrieval, or whether a markdown retrieval returns the
file's bytes or the document as archie parsed it.

Retrieving an image and a document in Portable Document Format is what makes
this tool's result shape a question rather than a detail: a tool call returns
structured data, not a byte stream on standard output. remargin answers it by
reporting a file's size through a separate metadata tool before anything fetches
it, returning a non-markdown file as an embedded resource carrying its media
type, and offering a downscaled fetch for images. That is remargin's answer, and
the requirements document still records "non-markdown files retrievable as
resources" as a candidate goal that Eduardo has not accepted.

## query

**Mirrors** [`archie query`](../cli/query.md). **Proposed, with a decided
starting point.**

Selects documents by an expression over their frontmatter, in a language that
also drives modification.

| Parameter | Type | Where it comes from |
|---|---|---|
| `exp` | string, required | `--exp='type=issue'`, Eduardo, 2026-08-30. His other spelling, of 2026-08-16, gives the same expression positionally, as `archie query 'tags contains "rule"' .grit/patterns/`. |
| `path` | string | Present in the 2026-08-16 spelling, absent from the 2026-08-30 one. Nothing states whether it defaults to the configured corpus root, whether it may be repeated, or whether it may be a glob. |

The two spellings are the reason this tool cannot be written down. A command
line distinguishes a positional argument from a named option; a tool call does
not, because every parameter is named. The tool has to fix one name for the
expression, and the corpus holds two spellings that no document reconciles.

**Returns.** Not specified. The grammar document's strawman — everything in it
Proposed, put there for Eduardo to react to — has the default output as one
value per line in JavaScript Object Notation (JSON), with a markdown form for
fragments. A tool that returns structured data has no line-oriented form, so
even the strawman does not carry over unchanged.

**Decided, about the command.** That a frontmatter query is the first thing
archie will do; that the language is to work like `jq`, with queries that chain
and with almost anything projectable; and that the same language drives
modification, one document at a time or in bulk. Those decisions are recorded on
the [`archie query`](../cli/query.md) page with the dates and documents that
settled them. None of them says anything about a tool.

## search

**Mirrors** [`archie search`](../cli/search.md). **Proposed, with a decided
scope.**

Matches a pattern against documents in a corpus.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string | `folder/` in Eduardo's one command line of 2026-08-30. |
| `pattern` | string, required | `"[0-9]?[a-z]*"` in the same line. The corpus does not state which regular-expression dialect it is written in, or whether it is a regular expression at all. |

**Decided.** Search runs over document bodies, over frontmatter when frontmatter
is specified, and over both when clauses are given for both. Eduardo stated it on
2026-08-31, and gave it as one of his reasons for archie writing its own parser.

**Unspecified.** How that scope is expressed. The short flags in his example,
`-g` and `-i`, are undefined; the question about them was asked on 2026-08-31 and
dropped when he answered the scope question instead, on the ground that the
syntax is not a requirement. So the tool's third parameter — whatever carries
body, frontmatter, or both — has no name and no shape in the corpus, and this
page does not invent one.

**Returns.** Not specified. Nothing states whether a match is reported as a file
path, a line, a line number, a fragment, or a document.

## write

**Mirrors** [`archie write`](../cli/write.md). **Proposed.**

Writes into the part of one document that a path expression addresses. It is the
only command in the corpus that operates on a sub-part of a document.

| Parameter | Type | Where it comes from |
|---|---|---|
| `file` | string, required | `folder/file.md` in Eduardo's command line of 2026-08-30. |
| `path` | string, required | `--path='# ##["Specs"][2]'` in the same line. The grammar of the expression is undefined; Eduardo said on 2026-08-31 that it needs its own extended discussion and that he does not have all the answers. |
| `append` | boolean | `--append` in the same line. Appending is the only mutation verb he has written down. |
| `from_body` | string | `--from-body='path-to-file.txt'`, the file the content is taken from. Nothing states whether content may be given inline instead. |

The two required parameters collide in the obvious naming: the document is
itself a path, and the option that addresses a part of it is spelled `--path`.
This page names the document `file` and leaves `path` to the expression, which
is the command line's own spelling. The corpus names neither.

**Returns.** Not specified. Nothing states what a write reports.

**Decided, about the command.** Every write is a transaction: a change produces
an object representing a change to a resource, modeled the way git operates, so
that a base document plus a chain of changes applied in order gives a
byte-identical latest version and unapplying the chain in reverse returns the
original. Eduardo stated the model on 2026-08-25. No document says whether a
tool call returns that object, a summary of it, or nothing.

## rm

**Mirrors** [`archie rm`](../cli/rm.md). **Proposed.**

Removes a file or a folder from a corpus.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string, required | A markdown file in one of Eduardo's two command lines of 2026-08-30, a folder in the other. |

No option is written down: no confirmation, no dry run, no recursion flag, no
force flag.

**Returns.** Not specified. Nothing states whether removing a folder removes its
contents, or what happens to documents elsewhere in the corpus that link to what
was removed.

## link

**Mirrors** [`archie link`](../cli/link.md). **Proposed.**

Turns mentions of a resource into links to that resource.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string, required | `folder/` in Eduardo's command line of 2026-08-30, the only spelling in the corpus. |

**Returns.** Not specified for archie. The program Eduardo pointed at, the
`link` subcommand of his `aa_tool` project, emits a report in JavaScript Object
Notation (JSON) listing every edit it made and every surface string it excluded
for being claimed by more than one resource. That is a property of `aa_tool`,
read from its source, and the [`archie link`](../cli/link.md) page records which
of its properties are and are not stated for archie. Its three options —
allowing a dirty git working tree, a dry run, and a report path — are not
written down for archie either.

**Decided, about the command.** That archie's `link` is almost the behavior of
`aa_tool`'s, and that in archie it is not restricted to people but works for any
resource. Eduardo said both on 2026-08-31, and said that what the "almost" is
waits for a feature discussion that has not happened.

## lint

**Mirrors** [`archie check`](../cli/check.md), which Eduardo's list spells
`archie lint`. **Root-design-only, fate open.**

Validates a corpus, or one path within it, and reports what it finds. It is the
most fully specified command in the root design document, which makes it the one
tool whose parameters and result shape are almost entirely quotable.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string | The optional argument in the root design's spelling; its default comes from the configuration file. |
| `format` | string, one of `text`, `json`, `markdown` | `--format`. Default `text`. |
| `fail_on` | string, one of `error`, `warning`, `any` | `--fail-on`. Default `error`. |
| `rule` | array of strings | `--rule`, which the root design's own example passes more than once. Whether it takes a rule name, a rule category, or either is not stated: that example passes two category names. |
| `ignore` | array of strings | `--ignore`, additional patterns beyond the ignore list in the configuration file. |

**Returns.** The validation report, which is the one result shape the corpus
specifies. Section 6.2 of the root design gives its JavaScript Object Notation
(JSON) form: the corpus root, the counts of files and assets scanned, the
findings split into errors, warnings, and information — each finding carrying
its rule, its path, its message, and whether it can be repaired — and a summary
of the counts. It is shown in full on the
[Output formats](../output-formats.md) page.

The `format` parameter is the one place where the mirroring produces something
odd. On a command line the three formats choose what a person or a pipeline
reads. A tool call returns structured data, so the JSON form is the result and
the other two are renderings of it.

**Which name the tool carries is unresolved.** The root design calls the command
`check`; Eduardo's list of 2026-08-30 has `lint` in that position and never says
`check`. This page uses `lint`, because that is the name Eduardo wrote, and
records that the specified behavior above is written under the other name.

**Deferred, about the rules.** Eduardo recorded on 2026-08-16 that the rule
catalog still encodes an earlier hard-coded model and is to be re-derived once
the schema language is designed. What a run reports therefore changes with the
schemas, even where the report's shape does not. See
[Validation rules](../validation-rules.md) and
[Shape language](../shape-language.md).

## init

**Mirrors** [`archie init`](../cli/init.md). **Proposed.**

Sets up a corpus: writes the configuration file and creates the folder index
files that are missing.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string | Optional in the root design's spelling; Eduardo's example passes `.` explicitly. The corpus does not state the default. |
| `config` | boolean | `--config`, create `archie.toml` if it is missing. |
| `indexes` | boolean | `--indexes`, create the missing `index.md` files. |
| `all` | boolean | `--all`, do both. |

Three booleans where the command line has three flags, one of which means the
other two. The corpus does not say what happens when none of the three is given.

**Returns.** Not specified. Nothing states what the command reports, or what a
created file contains beyond the note that a created index file carries the
properties inherited from its parents.

**Parked, about one of the two jobs.** The mechanism a folder index file serves
is under reconsideration: Eduardo said on 2026-08-16 that he wants something like
property inheritance but is not convinced the inherited-properties block in a
folder's `index.md` is the way, and parked the question. See
[Inheritance](../../concepts/inheritance.md).

## fix

**Mirrors** [`archie fix`](../cli/fix.md). **Root-design-only, fate open.**

Repairs the findings a validation run reports that can be repaired without a
person deciding anything.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string | The optional argument in the root design's spelling. |
| `dry_run` | boolean | `--dry-run`, show what would be repaired without changing any file. |
| `rule` | array of strings | `--rule`, repair only the findings of the named rule or rules. |
| `interactive` | boolean | `--interactive`, ask before each repair. |

**Returns.** Not specified. Nothing states what a repair run reports, or in which
format.

`interactive` is the parameter that does not survive the crossing. See
[the two commands that ask a person questions](#the-two-commands-that-ask-a-person-questions)
below.

## inbox

**Mirrors** [`archie inbox`](../cli/inbox.md). **Root-design-only, fate open.**

Reads the captures dropped into a corpus without being filed, asks a language
model to classify each one, and proposes where each should go.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string | The optional inbox path; the default comes from the configuration file. |
| `dry_run` | boolean | `--dry-run`, show the proposals without applying them. |
| `auto` | boolean | `--auto`, apply the proposals without asking. The root design marks this dangerous. |
| `provider` | string, one of `anthropic`, `openai` | `--provider`; the default comes from the configuration file. |
| `model` | string | `--model`; the default comes from the configuration file. |

The credential is read from the environment, from `ANTHROPIC_API_KEY` or
`OPENAI_API_KEY`. The corpus does not say which is consulted when both are set,
and does not say how an environment variable reaches a server the client
started.

**Returns.** Not specified. The root design illustrates an interactive prompt
offering approve, edit, skip, reject, and quit for each proposal; that is a
screen, not a result.

This command is also the one the direction Eduardo set on 2026-08-25 leaves out:
the responsibilities he placed in archie are the linter, the schema engine, the
document and schema abstractions, the transactional change model, and
backend-agnostic versioning, and classification with a language model is not
among them.

## trace

**Mirrors** [`archie trace`](../cli/trace.md). **Root-design-only, fate open.**

Prints the chain of inherited properties for one file: which folder index files
above it declare properties, what those properties are, and whether the file
carries them.

| Parameter | Type | Where it comes from |
|---|---|---|
| `file` | string, required | The required argument in the root design's spelling. |

The root design states no option for this command.

**Returns.** Not specified as data. The root design illustrates the output as an
indented tree, one block per folder index file, ending with two lines for the
file — what it must carry and what it does carry. A tool would return the chain
rather than the drawing of it, and the corpus does not describe the chain in any
other form.

**Parked, in addition.** The mechanism this command displays is itself under
reconsideration, so a tool that displays it cannot be more settled than the
mechanism.

## export

**Mirrors** [`archie export`](../cli/export.md). **Root-design-only, fate open.**

Serializes a corpus so another program can read it.

| Parameter | Type | Where it comes from |
|---|---|---|
| `path` | string | The optional argument in the root design's spelling; the default is not stated. |
| `include_content` | boolean | `--include-content`, include the content of each file. The root design warns the output becomes large. |
| `only_metadata` | boolean | `--only-metadata`, export only frontmatter, without the structure. |

The corpus does not say what happens when both are given.

**Returns.** The export document, which the root design describes in its
knowledge-patterns section: a list of files, each with its path, its
frontmatter, its observations — each carrying a category, its content, and its
tags — and its relations, each carrying a type and its targets; and beside that
list a graph in which each resource is a node and each relation is an edge
carrying its type. That is a description of a document the command writes, not
of a tool result, and the size warning is the reason the difference matters: a
result that a client pays for by the token is not the same thing as a file
written to disk.

This is the one command whose stated purpose names a language-model agent as its
consumer, which makes it the closest thing in the corpus to a reason for a
server. The corpus names it as a command all the same.

## import

**Mirrors** [`archie import`](../cli/import.md). **Root-design-only, fate open.**

Pulls items in from a source outside the corpus and writes them into it.

| Parameter | Type | Where it comes from |
|---|---|---|
| `source` | string, required | The required argument. `google-tasks` is the only value the corpus names. |
| `list` | string | `--list`, a specific task list named by its identifier. |
| `dry_run` | boolean | `--dry-run`, show what would be imported without writing. |
| `target` | string | `--target`, where imported items are written. Default: the inbox folder. |

**Returns.** Not specified. Nothing states what an imported item looks like once
written.

**Open in the corpus.** How authentication against Google Tasks is handled. The
root design lists it as one of its own two open questions and asks whether there
should be a separate command for signing in and where the resulting credentials
would live. A server makes the question sharper rather than easier: a tool call
has nowhere to put a browser sign-in.

## Commands with no tool

- **`archie help`.** The root design lists it with the other commands. It has no
  counterpart here: a client asks a Model Context Protocol server for its tool
  list and gets the tools with their parameters, so the help command has nothing
  to do. The command gets no page in this book either.
- **The command that starts the server.** A server does not expose the command
  that starts it. Which command that is has not been decided; the
  [server page](index.md) carries the question.
- **Structural operations.** Moving, renaming, archiving, and merging are
  Decided to be archie's territory and are Proposed in every detail. No document
  gives a command spelling for any of them, so no tool name can be derived by
  mirroring. See [Structural operations](../cli/structural-operations.md).

> **Suggestion (documentation author):** Eduardo would have to decide whether the
> structural operations get tools before they get command spellings, or after.
> The question is not idle, because the vision blueprint holds open whether the
> engine should be designed around remargin's interface from the outset with the
> command line as a thin shell over it. If it is, then remargin reaches these
> operations through the engine or through a server, and a command spelling is
> the last thing to be chosen rather than the first. If it is not, then the
> mirroring convention on this page decides the tool names as soon as the verbs
> are chosen. Either way this book invents no name for them.

## The two commands that ask a person questions

`archie fix` has an interactive mode that asks before each repair. `archie inbox`
shows each proposal and lets a person approve, edit, skip, or reject it. Both are
conversations, and a tool call is not one: the client sends parameters, the
server answers once.

> **Suggestion (documentation author):** Eduardo would have to decide what
> becomes of the two interactive commands on a tool surface. Three shapes are
> available and the corpus prefers none. The tool omits the interactive parameter
> and offers only the two non-interactive modes, a dry run and an unattended run.
> The one operation is split into two tools, one that proposes and one that
> applies a set of decisions it is handed, which is the shape remargin uses when
> it projects a mutating operation before running it. Or the commands get no tool
> at all, on the ground that both are the features whose fate the vision
> blueprint holds open. This is a decision about a surface that is not designed,
> for commands whose future is not settled.

## Where the corpus is silent

- **The tool list.** No document names one tool of archie's. The list on this
  page is the command inventory carried across by a mirroring convention, and it
  changes with the command line, which Eduardo has said can change.
- **Parameter names and types.** No document names one parameter of one archie
  tool. Every name on this page is either an option's name with its dashes
  replaced or the description of a positional argument.
- **Result shapes.** One command in the corpus has a specified output, the
  validation run, and it is specified as three formats a person or a pipeline
  reads rather than as a result a program consumes. Every other entry above says
  the result is not specified.
- **What a tool does that its command cannot.** Nothing states whether the
  server exposes anything beyond the commands — a change object handed back to
  the caller, a projection of a write before it is applied, a subscription to a
  document that changed. remargin's server carries a projection step of its own
  and leaves subscriptions out; archie's corpus addresses none of the three.

> **Suggestion (documentation author):** Eduardo would have to decide a default
> result shape for the tools whose command output the corpus does not specify —
> which is all of them but the validation run. remargin's answer is available and
> costs little to adopt: a result that carries repeated rows names its fields once
> and makes the rows positional, and a bounded page always reports the exact
> total so a partial answer cannot be read as the whole one. The reason to decide
> it once, rather than per tool, is the one Eduardo gave when he asked for short
> tool names on 2026-04-12: what the server sends costs the client tokens.

## Related pages

- [Model Context Protocol server](index.md) — where the statement comes from,
  remargin's conventions, configuration, and error mapping.
- [Command-line interface](../cli/index.md) — the command table these tools
  mirror, and the global options.
- [Output formats](../output-formats.md) — the validation report in all three
  forms.
- [Exit codes](../exit-codes.md) — the code table and what is known about it.
- [Query grammar](../query-grammar.md) — the expression language behind `query`,
  `search`, and the path given to `write`.
- [Structural operations](../cli/structural-operations.md) — the operations that
  have no spelling on either surface.
- [Design status](../../design-status.md) — the open questions, and the gap this
  page rests on.
