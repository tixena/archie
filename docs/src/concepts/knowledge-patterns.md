# Knowledge patterns

**Status:** Mixed — see markers.

Frontmatter describes a document as a whole. Knowledge patterns describe things
*inside* it: a fact worth extracting, a decision that was made, a typed connection
to another resource. They are ordinary markdown list lines that follow a
convention, so a person reading the document sees a bulleted list and archie sees
structured data. This page records the two patterns the design corpus defines, the
checks it proposes for them, and the exact status of all of it.

## The status of this whole area

**Decided.** That archie recognizes these patterns and can extract them is one of
the purpose bullets in the root design: knowledge patterns, meaning observations
and relations, are recognized and extractable. Eduardo asked in 2026-07-26 what
that bullet meant — "Please try to find out what this means. I don't remember.
Find from the data you have, not from inventing" — and acknowledged the answer,
which was drawn from the root design's own knowledge-patterns section.

**Proposed.** Everything below the bullet is the root design's own draft. Eduardo
has stated no position on the categories, the relation types, the syntax, or the
checks, and none of it has been revisited since the schema-driven direction of
2026-08-25 widened what schemas cover. Treat the specifics as written down and not
agreed.

## Observations

**Proposed.** An observation is a categorized fact written as a list line. The
syntax is a bracketed category, then the content, then an optional tag:

```markdown
- [fact] The project uses PostgreSQL 15
- [decision] We chose gRPC over REST for internal services
- [tip] Run migrations before deploying
- [warning] Never expose the admin endpoint publicly
- [question] Should we support multi-tenancy?
- [todo] Implement rate limiting
```

**Proposed.** The root design names eight categories and says the set is
extensible through configuration: `fact` for objective information, `decision` for
a choice that was made, `tip` for advice, `warning` for a caution, `question` for
something still open, `todo` for an inline action item that is not a task file,
`insight` for a realization, and `reference` for an external resource.

**Proposed.** The design distinguishes an observation from a tag, and gives the
reason directly: tags exist for discovery, so that a reader can find every note
about a topic, while observations are structured facts inside one document that
can be extracted and aggregated across a corpus, handed to a language model as
context, and queried.

Note that `todo` as an observation category sits beside a separate rule that every
task is its own document. The root design holds both: an inline `todo` observation
is not a task, and a note carrying several independent checkbox items is reported
so that it can be split into task files.

## Relations

**Proposed.** A relation is a typed connection between resources, written as a
list line whose first word is the relation type and whose targets are wiki-links:

```markdown
- depends_on [[Authentication Service]]
- blocks [[Mobile App Release]]
- owned_by [[Dylan Minaya]]
- related_to [[API Design]] [[Security Review]]
- supersedes [[Old Proposal]]
```

**Proposed.** One line may name more than one target. The root design lists nine
relation types, each with a defined inverse, and says the set is extensible
through configuration: `depends_on` inverts to `dependency_of`, `blocks` to
`blocked_by`, `owned_by` to `owns`, `part_of` to `contains`, `supersedes` to
`superseded_by`, `references` to `referenced_by`, `implements` to
`implemented_by`, `extends` to `extended_by`, and `related_to` inverts to itself.

**Proposed.** Both patterns may appear anywhere in a document. The root design
notes a convention rather than a rule: grouping them under their own headings,
with the observations in one section and the relations in another, ahead of the
document's prose.

## What "recognized" means

**Proposed.** Recognized means archie parses the patterns and can check them.
Three checks are defined, each with a severity that configuration can set to off,
informational, warning, or error.

- **Observation format.** An observation line must follow the bracketed-category
  form. A malformed line is reported with the file, the line number, what was
  found, and what was expected. The root design gives it the warning severity.
- **Relation target exists.** A relation's target must exist as a file in the
  corpus. A target that resolves to nothing is reported with the file, the line
  number, and the line itself. Warning severity.
- **Relation type is known.** A relation type should come from the known set or
  from the additions configuration declares. An unknown type is reported
  informationally, with the known types listed as alternatives.

**Proposed.** Configuration carries the two extensible lists and the three
severities in one block, alongside a switch that turns the whole area off.

## What "extractable" means

**Proposed.** Extraction is the job of `archie export`. It emits, for each file,
that file's frontmatter, its observations with their categories and tags, and its
relations with their types and targets. Alongside the per-file entries it emits a
graph: a list of nodes, and a list of edges each carrying a source, a target, and
the relation type that connects them.

**Decided.** The fate of that command is open. `archie export` appears in the root
design and is absent from the list of commands Eduardo wrote on 2026-08-30, and
what becomes of the root design's vault-application commands is one of the
program's unanswered questions. The [archie export](../reference/cli/export.md)
reference page records the same status.

## Where the idea came from

**Proposed.** The root design names its source: Basic Memory, an open-source
project at `basicmachines-co/basic-memory`. Basic Memory is a knowledge-management
system that gives conversations with a language model a memory that outlives them,
by keeping the knowledge in local markdown files that both the person and the
model read and write, connected through the Model Context Protocol (MCP) — the
same protocol archie is to expose a server for. Its files carry frontmatter, and
its two in-content patterns are the ones described above: categorized observations
and typed relations between entities.

**Proposed.** The root design's comparison sets out where the two part ways rather
than where they meet. archie's goal is to enforce structure and validate; Basic
Memory's is to build knowledge out of conversations. In archie a language model is
an optional helper used for classifying inbox captures; in Basic Memory the model
is the mechanism. archie writes files only when explicitly told to; Basic Memory
writes continuously while a conversation runs. The comparison closes by naming
them complementary rather than competing, since both store plain markdown.

## Gaps

**Not in the corpus.** How a wiki-link target is resolved to a file. The check
that relation targets must exist reports that no file matched a name, without
saying how a name is matched against the corpus — by filename, by a title in
frontmatter, by a path, or by something else.

**Not in the corpus.** How these patterns survive the schema-driven direction.
Shapes describe what a document or a part of one must look like, and observations
and relations are content inside a document with a required form. Whether the
pattern checks become shapes, stay as separate built-in rules, or disappear is not
addressed anywhere.

**A contradiction in the corpus.** The root design says both that archie has no
knowledge graph — its comparison table gives archie an inheritance tree only,
against Basic Memory's semantic knowledge graph — and that `archie export` emits a
graph of nodes and edges built from typed relations. The two statements are in the
same document and this book does not reconcile them; a reader should treat the
graph question as unsettled until Eduardo says which is meant.
