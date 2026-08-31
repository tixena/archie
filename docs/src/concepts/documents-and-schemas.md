# Documents and schemas

**Status:** Mixed — see markers.

archie's two core objects are the markdown document and the schema. Neither is a
file. A file on disk is one way a document can be stored and one place a schema
can be written down, but the engine works on the objects, not on the bytes. This
page records what the design corpus says about those two abstractions, about the
parser that produces them, and about who is allowed to reformat what.

## The document and the schema are abstractions, not files

**Proposed.** In Eduardo's words, on 2026-08-25: "Needless to say, in archie I
must abstract away the concept of a markdown file/object, and also the schemas."
The engine's core types are the markdown document as an object and the schema as
an object, neither tied to a file on disk. The archie feature ledger holds this
with the status `idea`, and the requirements document holds it among the candidate
goals awaiting Eduardo's acceptance, which is why it is marked Proposed rather
than Decided. What is settled is where it belongs: the vision blueprint's record
of the 2026-08-25 boundary places the document and schema abstractions in archie.

The ledger states why the abstraction comes first rather than last: it is the
prerequisite for the two capabilities built on top of it. Change objects
(see [Changes and versions](changes-and-versions.md)) describe a change to a
resource, not an edit to a path. Version tracking over pluggable storage backends
(see [Backends and capabilities](backends-and-capabilities.md)) only makes sense
if a document can exist without a filesystem underneath it.

**Not in the corpus.** What the document object contains — how a heading, a list,
a table, a code fence, or a remargin comment block is represented, and what
identifies a document when there is no path — is not written down anywhere. The
closest the corpus comes is the grammar document's strawman, which sketches a
document as frontmatter plus a body tree in which a heading nests the blocks under
it until the next heading of equal or higher level. That sketch is explicitly a
strawman for Eduardo to argue with, not a design.

## The frontmatter engine is the foundation

**Decided.** The frontmatter engine is not one feature of archie among others; it
is what everything else stands on. The frontmatter and architecture discussion
makes the case by walking the command set: validating reads and checks fields,
repairing reads and modifies them, classifying an inbox capture writes them from
scratch, tracing reads them up a chain of folders, and exporting serializes them.
Without the frontmatter layer, none of those exist.

That document also records where archie started. The problem that began it was
small and concrete: finding, among the files in a `.grit/patterns/` folder, the
ones whose frontmatter carries a `"rule"` tag. It is done today with a grep over
the text of the frontmatter, which has no understanding of the frontmatter as data
and breaks whenever its formatting changes. The survey in that document found no
standalone command-line tool that queries markdown files by frontmatter field —
`yq` extracts frontmatter from one file but needs shell glue to filter across
many, and Obsidian's Dataview does exactly this but only inside Obsidian.

**Decided.** The first thing to build is that query. Eduardo confirmed on
2026-08-16: `archie query 'tags contains "rule"' .grit/patterns/`, a command that
walks a directory, parses each file's frontmatter, evaluates one expression
against it, and prints the paths that match — no configuration file, no rules, no
inheritance. He added that he would give the signal to start, because he wanted to
dump other thinking out of his head first. He has not given it. Whether the
remargin-core decision changes that starting point is one of the program's open
questions.

**Decided.** archie is built as a Cargo workspace of library crates with a thin
command-line crate on top, rather than as a single program with internal modules.
Eduardo chose it on 2026-08-16 in the same discussion. Two things about it are
open and recorded as such: whether the workspace exists from the first commit or
is extracted once the module boundaries show themselves, and which crates it
holds. The crate list sketched in that discussion predates both the 2026-08-25
direction and the decision that remargin is the consumer, so it is to be
re-derived rather than used.

## archie writes its own parser

**Decided.** archie parses markdown with a parser of its own. Eduardo settled it
on 2026-08-30: "We write our own parser. May be inspired on those libraries but
it's our own stuff." The libraries in question were the ones an earlier research
list proposed evaluating — `gray_matter` for frontmatter extraction, the YAML
crates for the frontmatter body, `comrak` for markdown. They are reference
material now, not candidates.

**Decided.** The parser's scope covers both halves of a document. Asked whether
"our own parser" meant only the frontmatter framing or also the YAML parser and
the markdown body parser, Eduardo answered on 2026-08-30: "yes, includes the yaml
parser (for the frontmatter) and the markdown parser. I have my reasons. I require
this parser to be really good so we will be establishing performance goals for
it." The grammar document carries this as its first numbered decision.

**Decided.** He named one of those reasons himself on 2026-08-30. Search has to
run over document bodies, over frontmatter when frontmatter is specified, and
over both when clauses are given for both — and that, he said, is one of the
reasons he needs to write his own parser.

**Not in the corpus.** The performance goals. Eduardo will state them; until he
does, nothing about the parser is testable and this book makes no claim about how
fast anything is.

## Whose formatting wins

**Decided.** archie may respect the formatting it finds; remargin does not.

The corpus records this as a decision that was made once and then narrowed. On
2026-08-16, answering a research note about YAML parsers losing comments and
reordering keys when they round-trip a document, Eduardo wrote: "My YAML will be
standard but will be very opinionated. I won't respect anyone's format the same
way that remargin doesn't. I may allow some 'linting rules' in the config and
that's it. After that every resource in a vault must respect the lint rules." On
2026-08-30 he narrowed it: "I can retract this a bit. Archie, as an agnostic
markdown tool, can respect others but definitely remargin wouldn't."

So the two halves land in different places. Writing a canonical form — a fixed key
order, no preserved comments, one spelling for everything — is remargin's write
path. Reading a document written in someone else's style and giving it back in
that style is a capability of archie's own parser, available because the parser is
archie's own code rather than a general-purpose YAML library. Within a vault that
remargin governs, every resource still has to satisfy the lint rules that
configuration allows.

One consequence is recorded with it: the round-tripping experiment that the
earlier research proposed is closed. It was aimed at `serde_yaml`, which stopped
receiving releases after a version published in March 2024 and is marked
deprecated, and it is moot in any case now that the parser is archie's.

**Not in the corpus.** What the canonical form actually is — key order, quoting,
indentation, how a multi-line value is written — is not specified anywhere, and
neither is the list of lint rules that configuration may allow. This book states
no canonical form.

## Where schemas come from and what they cover

**Decided.** Schemas rule both halves of a document. The root design's purpose
list records Eduardo's 2026-07-26 answer: every markdown file may or may not have
frontmatter, depending on what the rules say, and validating files against schemas
covering both frontmatter and structure is one of archie's central features.
Whether frontmatter is required at all is a consequence of the governing schema,
not a fixed rule — as is which properties are required, so it is not necessarily
`type` and `created`, which the earlier draft hard-coded.

**Decided.** The consequences of that have not been worked through yet, and the
corpus says so in a deferral note at the top of the root design's validation-rule
catalog. That catalog, the rule and resource-type blocks in the configuration
section, and the list of what automatic repair can fix all still encode the older
hard-coded model: a global rule requiring frontmatter with the inbox as the one
exception, `type` and `created` as fixed requirements, and a fixed list of
resource types. Each becomes a consequence of what the schemas say. The rework
waits for the schema language to be designed, and it will be done against the
wider 2026-08-25 direction — shapes for a document or any sub-part of one, in a
strict or a loose mode — rather than against the frontmatter-only picture the old
rules assume.

**Not in the corpus.** The schema language itself: how a schema is written, how a
schema attaches to a document or to a folder, and what a schema can say about
structure as opposed to frontmatter. The vocabulary question of how a schema
relates to a shape and to a resource type is taken up on
[Shapes, schemas, and resource types](shapes-schemas-resource-types.md).

## Documents that are not markdown

**Proposed.** Two of the commands Eduardo wrote down on 2026-08-30 retrieve an
image and a PDF file, written as `archie get folder/image.png` and
`archie get folder/doc.pdf`. That makes non-markdown files into resources archie
can hand back, which the root design had touched only from the other side, through
a rule
warning about assets that no markdown file references. The requirements document
holds it as a candidate goal, not an accepted one, and nothing anywhere describes
what archie does with such a file beyond returning it.
