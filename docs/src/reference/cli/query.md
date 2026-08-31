# archie query

**Status:** Mixed — see markers.

`archie query` selects documents by an expression and prints, or modifies, what
the expression picks out. It is the command the design corpus says the most
about, and the one whose spelling the corpus says the least about: the direction
of its language is settled, and the language itself is not designed.

## Synopsis

Two spellings appear in the corpus, both written by Eduardo Burgos, and no
document reconciles them.

```console
$ archie query 'tags contains "rule"' .grit/patterns/
$ archie query --exp='type=issue'
```

The first is from 2026-08-16, on the frontmatter and architecture discussion,
where the expression and the path are positional arguments. The second is from
2026-08-30, on the planning document, where the expression is the value of a
named option and no path appears. The grammar document lists the spelling of the
language — operators, quoting, escaping, and how an expression is written as a
command-line argument as opposed to inside a pipeline — among the things that are
still unspecified.

## Arguments and options

The corpus specifies neither. The two command lines above are the whole of what
is written down: an expression, given positionally in one and as `--exp=` in the
other, and a path, present in one and absent in the other. Nothing states whether
the path defaults to the configured corpus root when it is omitted, whether more
than one path may be given, or whether a path may be a glob.

## What is decided

**Decided.** A frontmatter query is the first thing archie will do. Eduardo
confirmed on 2026-08-16, on the frontmatter and architecture discussion, that
`archie query 'tags contains "rule"' .grit/patterns/` is the first thing to
build, and that starting is on his signal because he wanted to write other things
down first. He has not given the signal. Whether the decision to make archie the
core of remargin changes that first slice is an open question in the vision
blueprint.

**Decided.** The direction of the language. Eduardo stated it on 2026-08-16 on
the planning document and restated it on 2026-08-30: something like `jq`, with
queries that chain and with almost anything projectable. In his words, "I'd like
something like `jq` because it is extremely powerful and also I'd like to be able
to chain queries and be able to project almost anything."

**Decided.** The same language drives modification, one document at a time or in
bulk. His example, from the same statement: append a section carrying the current
date to each of the elements a query returns. Modification through the query is
what makes the language address parts of a document rather than whole documents.

**Decided.** The parser and the language are archie's own code. Recorded as
decision G1 on the grammar document, from the planning document of 2026-08-30:
archie writes its own parser for both the frontmatter and the markdown body, and
Eduardo will establish performance goals for it. Existing crates may inspire it;
none is depended on for parsing.

**Decided, as a withdrawal.** An earlier recommendation on the planning document —
named flags for common fields (`archie query --type project --status active`)
plus a `--where` option carrying an arbitrary expression — was made by an agent
and is explicitly withdrawn in favor of the language above. No page in this book
revives it.

## What is proposed

**Proposed.** Everything in section 3 of the grammar document is a strawman, put
there for Eduardo to react to, and nothing in it is decided until it is rewritten
into that document's decisions section. The strawman says:

- One expression language serves `archie query`, `archie search`, and the
  `--path` option of `archie write`; the verb fixes what the expression runs
  against and what comes out.
- A document is frontmatter, which is a mapping, plus a body tree in which a
  heading nests the blocks under it — paragraphs, lists and their items, tables,
  code fences, and remargin comment blocks — until the next heading of equal or
  higher level.
- Frontmatter is selected with comparisons such as `fm.type == "issue"` and
  `fm.tags has "rule"`; clauses combine with `and`, `or`, and `not`.
- Stages join with `|`, as in `jq`, and each stage receives the previous stage's
  selection. A projection names what comes out: `{path, fm.title}` for values,
  `.body` for a fragment.
- The default output is one value per line in JavaScript Object Notation (JSON),
  with `--format md` for fragments as markdown.
- A mutation verb applies to a selection: `append`, `prepend`, `set`, `remove`,
  `replace`. One invocation is one transaction covering every document touched;
  the alternative recorded alongside it is one transaction per document.

The full strawman, with the statement of Eduardo's that each element derives
from, is on the [Query grammar](../query-grammar.md) page.

## Behavior

What a run does, to the extent the corpus states it: archie walks the given path,
finds markdown files, parses the frontmatter of each with its own parser, and
produces the documents the expression selects. The smallest form of this — walk a
tree, parse frontmatter, print the paths of the files that match — is what the
frontmatter and architecture discussion describes as the first thing to build.

Modification through a query is a change, and every change through remargin is a
transactional object: given a base document and a chain of changes applied in
order, the result is byte-identical to the latest version, and unapplying the
chain in reverse returns the original. Eduardo stated the change model on
2026-08-25; it is recorded in his feature ledger. Whether one query-driven mass
modification is one such object or one per document touched is unspecified.

**Decided.** archie can respect the existing formatting of frontmatter it did not
write. Eduardo said on 2026-08-31, amending a stricter statement of 2026-08-16,
that archie as a markdown tool that stays agnostic can respect other people's
format, while remargin definitely will not: the opinionated canonical form is
remargin's write path, and preserving an existing format is a capability of
archie's own parser.

## Examples

```console
$ archie query 'tags contains "rule"' .grit/patterns/
$ archie query --exp='type=issue'
```

Both are Eduardo's own, quoted from the corpus. The design specifies no output
for either, so none is shown. Any further example would be invented, because the
grammar that would make one meaningful is not designed.

## Exit codes

The corpus gives one exit-code table, in the root design document's
error-handling section, and it is written in the language of the validation run:
success, validation errors, warnings only, configuration error, file system
error, and language-model provider error. It does not say which of those a query
run can return, or what a query with no matches returns. See
[Exit codes](../exit-codes.md).

## Related pages

- [Query grammar](../query-grammar.md) — the strawman in full and the list of
  what is unspecified.
- [archie search](search.md) — matching against document bodies and frontmatter.
- [archie write](write.md) — the path expression that addresses part of one
  document.
- [Change objects](../change-objects.md) — the transactional model a
  query-driven modification produces.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
