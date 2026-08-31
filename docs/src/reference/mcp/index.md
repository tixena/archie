# Model Context Protocol server

**Status:** Mixed — see markers.

archie exposes a server that speaks the Model Context Protocol (MCP): the
protocol through which a language-model client calls a program's operations as
structured tool calls, instead of running the program in a shell and reading the
text it prints. Eduardo Burgos said in chat on 2026-08-31 that archie has one,
"just like remargin". That sentence, with its date, is the whole of the source
for this page and for the [tools](tools.md) page beside it: no document in the
design corpus records the server at all.

## Where this page comes from

**Decided, stated outside the design corpus.** archie exposes a Model Context
Protocol server. Eduardo said so in chat on 2026-08-31. No document in archie's
design corpus records it: the root design document's module layout in section
7.1 has no server module, its command list in section 2.1 names eight commands
and none of them starts a server, and the list Eduardo wrote on 2026-08-30 of
the commands he would actually run does not name one either. This book carries
the statement with its date wherever it appears, and the gap is listed on the
[Design status](../../design-status.md) page.

**Open in the corpus.** Whose server this is. The program's vision blueprint,
setting out what the decision that archie becomes the core of remargin leaves
unanswered, asks which of remargin's present subsystems — threaded comments,
identities and signatures, permission realms, the Model Context Protocol server,
the command line, and the Obsidian plugin — stay remargin's and which become
archie's. Eduardo's statement gives archie a server of its own; it does not
settle that question, and no page in this book settles it.

**How to read the rest of this page.** Statements below are of three kinds, and
each says which it is. **Root design** marks something archie's root design
document states about the command line or the configuration, carried here
because a server has to answer the same question. **Proposed** marks a reading
of the phrase "just like remargin" against remargin's own server. A
**Suggestion** is this book's own addition, and it names the decision it asks
Eduardo for. Nothing about archie's server is Decided beyond the fact that there
is one.

## What remargin's server does

This section is an observation about remargin, not a statement about archie. It
is recorded because "just like remargin" is the only instruction the corpus
gives about archie's server, so what remargin does is the entire content of that
instruction. It is read from remargin's Model Context Protocol interface
document of 2026-04-12 and the task specification that implements it, both in
remargin's design corpus beside archie's, and from the tools the installed
remargin server exposes.

- **One command starts it.** `remargin mcp` starts a server that speaks the
  protocol over standard input and standard output rather than over a network
  port. A client is configured with the program's name and that one argument.
- **Its tools are its commands.** The stated goal of the task that implements
  the server is that it exposes the same operations the command line does, each
  tool mapping to one library function. The task lists sixteen of them, among
  them `comment`, `ack`, `ls`, `get`, `write`, `query`, and `lint`.
- **Tool names are short and carry no prefix.** Settled in a thread on the
  interface document on 2026-04-12. The agent who wrote that document had
  prefixed every tool with the program's name and asked whether that was right.
  Eduardo answered: "About tool naming I'd rather have them short. MCP usage is
  expensive." The reply recorded the outcome — short names without a prefix,
  because a server already scopes the tools it exposes, and the tools mirror the
  command-line commands anyway. So the tool is `get`, not `remargin_get`. A name
  that needs more than one word joins them with an underscore, as in
  `get_image`, `sandbox_add`, and `permissions_show`.
- **The command line was designed first, deliberately.** In the same thread
  Eduardo deferred the whole server design: "I'll let you decide on that after we
  finish with CLI" — his shorthand for the command-line interface. The reply
  recorded why deferring cost nothing: the tools mirror the commands anyway.
- **Payloads are compact and columnar.** A result carrying repeated rows names
  its fields once and makes every row positional. The search tool returns the
  total, a list naming the four fields of a match — the line, whether the match
  sits in the document body or in a comment, the matched text, and the comment
  identifier where there is one — and then the files, each with its matches as
  bare arrays in that order. The reason is the one Eduardo gave for short names:
  what the server sends costs the client tokens, so a field name is written once
  instead of once per row.
- **A bounded page still reports the exact total.** Search takes a page size and
  an offset, and every answer carries the exact number of matches in the corpus
  together with the size the page was clamped to, so a partial answer cannot be
  read as a complete one. A separate tool exists for the case where a result
  overflowed the client's own limit; it lowers the page size for the rest of the
  session.
- **Anything that writes can be projected first.** A `plan` tool takes the name
  of a mutating operation and the parameters it would be given, and returns what
  would happen — whether it would commit, what it would refuse and why, which
  line ranges would change — without touching the disk.
- **The identity is resolved once, when the server starts.** A `whoami` tool
  reports it, and answers that none was found rather than failing when no
  identity is configured.
- **The root is resolved when the server starts too.** Path parameters default
  to the directory the server was started in, and the set of directories it will
  serve is fixed at start, from the permissions block of the configuration file
  found by walking up from that directory. Editing that file while the server
  runs does not move its roots; restarting it does.
- **Errors carry the path and the reason.** Every error names the file it
  concerns, a failure inside a batch names the operation that failed, and a
  non-zero exit status becomes a protocol error response.
- **Binary content has a path of its own.** A metadata tool reports a file's
  size before anything fetches it, the read tool returns a non-markdown file as
  an embedded resource carrying its media type, and an image can be fetched
  downscaled to fit the client's budget.

remargin's design leaves two features of the protocol out: notifications, by
which a server tells a client that something changed, and resources, by which a
document is exposed as something the client watches rather than asks for. Both
are recorded as deferred, and the server exposes tools only.

## Starting archie's server

**Proposed.** One command starts the server, and it speaks the protocol over
standard input and standard output. This mirrors remargin and rests on nothing
else.

```console
$ archie mcp
```

> **Suggestion (documentation author):** the command line above is this book's
> invention. Neither the root design's command list nor Eduardo's list of
> 2026-08-30 has an entry that starts a server, because both predate the
> statement that archie has one. Eduardo would have to decide the spelling and
> the shape: a subcommand named `mcp`, mirroring `remargin mcp`; an option on the
> program; or a second binary, which is a shape the chosen project layout — a
> Cargo workspace of library crates with a thin command-line crate on top — makes
> available. Until he decides, no page in this book shows a client configuration
> for archie.

## Configuration and the corpus root

**Root design.** archie reads a configuration file named `archie.toml` at the
repository root, and section 5.2 gives the order in which settings take
precedence, highest first: options given on the command line, then environment
variables, then the file, then built-in defaults. Section 5.3 names the two
variables that bear on a server, `ARCHIE_CONFIG` for the path to the
configuration file and `ARCHIE_ROOT` for the corpus root. What the file holds is
documented on the [Configuration](../config.md) page.

**Proposed.** A server resolves its configuration the same way and at the moment
it starts, from the directory it was started in, which is how remargin fixes its
roots. Nothing in archie's corpus says so.

That precedence order was written for a run of a command that starts, does one
job, and exits. A server outlives every call it serves, and the highest entry in
the order — options given on the command line — has no counterpart inside a tool
call.

> **Suggestion (documentation author):** Eduardo would have to decide how the
> precedence order reads for a server that keeps running. Three points need an
> answer. Whether the parameters of a tool call stand where command-line options
> stand, at the top of the order, so that a call may name a corpus root the
> server was not started in. Whether the configuration file is read once at start
> or re-read when it changes, given that the rules a validation run applies are
> configured in it. And whether one server may serve more than one corpus, which
> the root design already holds open as a question of its own when it asks
> whether one configuration may cover more than one vault.

## How tools map to commands

**Proposed.** One tool per command, named as the command is named, taking the
command's arguments and options as its parameters. That is remargin's
convention, and Eduardo's phrase carries it over. The [tools](tools.md) page
works it through one command at a time.

Two things in archie's own corpus keep the mapping from being mechanical, and
neither is resolved.

- **A command with two names.** The validation run is `archie check` in the root
  design document and `archie lint` in Eduardo's list. A command line can carry
  both spellings, one as an alias of the other; a tool list has to choose the
  name the client sees, or expose one operation twice.
- **A command with two shapes.** `archie query` is written twice by Eduardo,
  once with the expression and the path as positional arguments and once with the
  expression as the value of a named option and no path at all. The distinction
  does not survive into a tool call, where every parameter is named, so the tool
  has to fix the names those two spellings disagree about.

Under both sits the larger reason the tool list cannot be settled: it mirrors a
command line that is not settled. Eduardo stated on 2026-08-30 that his command
list is not written in stone, that the verbs and the syntax can change and are
not a hard requirement. When he deferred remargin's server design he set the
same order of work — the command line first, the tools after — and the reply
recorded why that order is safe: the tools mirror the commands anyway.

## Payload conventions

**Proposed.** Results are compact and columnar in the way remargin's are: a
result carrying repeated rows names its fields once and makes the rows
positional, and a bounded page reports the exact total so it cannot be read as
the whole answer.

**Root design.** The corpus specifies the output of exactly one command. Section
6 gives three formats for the validation run — plain text, which is the default,
JavaScript Object Notation (JSON), and markdown — selected with `--format`, with
a default in the configuration file's output block. The JSON form is one
document holding the corpus root, the counts of files and assets scanned, the
findings split into errors, warnings, and information, and a summary of the
counts. It is shown in full on the [Output formats](../output-formats.md) page.

No other command in archie's corpus has a specified output shape, so no other
tool has a specified return shape. The [tools](tools.md) page says so for each
one rather than inventing a result.

## Errors and exit codes

**Root design.** Section 10.1 gives one exit-code table for the program: 0 for
success with no issues, 1 for validation errors found, 2 for warnings only when
`--fail-on warning` was given, 3 for a configuration error, 4 for a file system
error, and 5 for a language-model provider error. The table is written in the
language of the validation run, and the corpus does not restate it for any other
command. The [Exit codes](../exit-codes.md) page carries it in full.

**Proposed.** A non-zero exit becomes a protocol error response, and every error
names the path it concerns. That is remargin's rule, carried over by Eduardo's
phrase.

Carried over literally, the rule turns a report into a failure. Codes 1 and 2
are the outcomes of a validation run that did exactly what it was asked to do:
it found errors, or it found warnings and had been told to treat them as a
failure. On a command line that distinction is a matter of what the shell does
next; in a tool call, a client reads an error response and a result differently.

> **Suggestion (documentation author):** Eduardo would have to decide which exit
> codes become protocol errors. One reading is that 3, 4, and 5 — the
> configuration, file system, and provider failures — are errors, because the
> call could not be served, while 0, 1, and 2 are results, because the call was
> served and the findings are the answer, with the counts already in the report.
> The other reading is that the tool reproduces the exit code faithfully and
> leaves the client to decide. The corpus chooses neither, because it does not
> record that the server exists.

## Where the corpus is silent

- **The server has no design document.** Everything above is either an
  observation about remargin or a reading of one sentence of Eduardo's. No
  document states archie's tool list, its parameter names, its result shapes, or
  its transport.
- **Nothing states who may call it.** remargin's server serves only the paths it
  was started with and refuses operations its permissions deny. Eduardo stated on
  2026-08-31 that the permission system — view or edit access to files and even
  to parts of markdown documents, so that only the accessible portion is shown —
  is remargin's, and the note recording it says plainly that how archie serves it
  mechanically is not designed anywhere. Whether archie's own server enforces
  anything is unstated.
- **Nothing states how a session is bound to a corpus.** archie's configuration
  names a corpus root and its environment carries `ARCHIE_ROOT`, but no document
  says whether a server serves one corpus for its lifetime or takes a corpus per
  call.
- **Nothing states what the server is for.** The root design names a
  language-model agent as the consumer of `archie export`, which is the closest
  the corpus comes to naming an audience for a machine-readable surface, and it
  names that as a command rather than as a server.

## Related pages

- [Model Context Protocol tools](tools.md) — one entry per tool, with the
  command each one mirrors.
- [Command-line interface](../cli/index.md) — the surface these tools mirror,
  its global options, and its command table.
- [Configuration](../config.md) — the `archie.toml` file and the precedence
  order.
- [Exit codes](../exit-codes.md) — the code table and what is known about it.
- [Output formats](../output-formats.md) — the one output shape the corpus
  specifies.
- [Use archie from an agent](../../guides/use-from-an-agent.md) — the
  task-shaped guide to calling archie from a program.
- [archie and remargin](../../concepts/archie-and-remargin.md) — the boundary
  between the two, and the open question about which subsystems belong to which.
- [Design status](../../design-status.md) — what is decided, what is proposed,
  and the gaps between this book and the corpus, this page's included.
