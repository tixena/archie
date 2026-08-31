# Query grammar

**Status:** Mixed — see markers.

This page renders the grammar document from archie's design corpus, a mini
request-for-comments on the query, path, and search expressions. That document
was restructured on 2026-08-31 at Eduardo Burgos's request, because he has
nothing concrete yet and wanted a document that starts the discussion. Its design
section is a strawman — a first concrete proposal to argue with, built only from
things he has said, and settled by nobody. Everything in this page's strawman
sections is therefore **Proposed**, and nothing about the grammar itself is
decided.

## What is settled

Four points are recorded as decisions on the grammar document. Each is numbered
there and each carries the date and the place Eduardo stated it.

**Decided.** The parser is archie's own, and that includes the parser for the
frontmatter and the parser for the markdown body. He settled the direction on
2026-08-30 on the planning document — "We write our own parser. May be inspired
on those libraries but it's our own stuff" — and the scope on 2026-08-31: "yes,
includes the yaml parser (for the frontmatter) and the markdown parser. I have my
reasons. I require this parser to be really good so we will be establishing
performance goals for it." The performance goals are his to state and he has not
stated them, so this book makes no performance claim.

**Decided.** archie can respect other people's YAML formatting; remargin's write
path writes archie's canonical form. He stated the opinionated half on 2026-08-16
— "My YAML will be standard but will be very opinionated. I won't respect
anyone's format the same way that remargin doesn't" — and amended it on
2026-08-30: "Archie, as an agnostic markdown tool, can respect others but
definitely remargin wouldn't."

**Decided.** Search runs over document bodies, over frontmatter when frontmatter
is specified, and over both when clauses are given for both. He said it on
2026-08-30 on the planning document, and named it as one of his reasons for
writing archie's own parser.

**Decided.** The command list he wrote on 2026-08-30 is not a requirement. On
2026-08-31: "any of those commands can change. It is not written in stone. You
told me for 10 things that come to my mind but the syntax is not a hard
requirement. We can discuss about it." Every command spelling in this book carries
that qualification; the full list is in the
[Command-line interface](cli/index.md) reference.

## The expressions that exist

**Decided as the record of what he has typed.** Four expressions exist in the
corpus, and the grammar document notes that their syntax is explicitly not a hard
requirement. They are the whole of the written evidence.

```console
$ archie query 'tags contains "rule"' .grit/patterns/
$ archie query --exp='type=issue'
$ archie write folder/file.md --path='# ##["Specs"][2]' --append --from-body='path-to-file.txt'
$ archie search folder/ -gi "[0-9]?[a-z]*"
```

The first is from 2026-08-16 and is the first thing Eduardo said he wants built;
the other three are from his command list of 2026-08-30.

**Decided as the reason for the work.** What the grammar has to be, in his words
from 2026-08-16, restated 2026-08-30: "I'd like something like `jq` because it is
extremely powerful and also I'd like to be able to chain queries and be able to
project almost anything", and he wants to "modify resources even punctual or
massively (like for example 'append this section with the current date to each of
the elements from this query')", and "We need to build a strong parser and a
strong DSL for that. I don't mind spending lots of times planning." An earlier
recommendation on the planning document — named flags for common fields plus a
`--where` expression — was withdrawn when he said this.

How the same needs are met today, as the grammar document records: by `grep` over
the text of frontmatter, and by remargin's own query, search, and write
operations, which address files and line ranges rather than document structure.

## The strawman

Everything in this section is **Proposed**. It is the grammar document's design
section, and that document states that nothing in it is decided until it is
rewritten into its decisions section.

### One expression language, three entry points

`query`, `search`, and the path given to `write` share one grammar; the verb
fixes what the expression runs against — a set of documents, or one document —
and what comes out. The alternative the document records alongside it: a
document-set query and a within-document path as two grammars joined at a defined
seam.

### The document model the grammar walks

A document is frontmatter, which is a mapping, plus a body tree. In the body
tree, a heading nests the blocks under it — paragraphs, lists and their items,
tables, code fences, remargin comment blocks — until the next heading of equal or
higher level.

### Selecting

Three kinds of clause, drawn from the three kinds of expression Eduardo has
written:

```
fm.type == "issue"
fm.tags has "rule"
body ~ /[0-9]?[a-z]*/
```

and a body position written as a path of heading steps followed by an index. The
strawman reads his `# ##["Specs"][2]` as the level-one heading, then the
level-two heading titled "Specs" beneath it, then the second child block of that
heading. Clauses combine with `and`, `or`, and `not`.

### Chaining and projecting

Stages join with `|`, in the manner of `jq`, and each stage receives the previous
stage's selection. A projection names what comes out: `{path, fm.title}` for
values, `.body` for a fragment. The strawman's default output is one JSON value
per line, with a markdown format available for fragments.

### Mutating

A mutation verb applies to a selection: `append`, `prepend`, `set`, `remove`,
`replace`. The only verbs in Eduardo's own material are appending — his `write`
example uses `--append` with `--from-body`, and his stated wish is to "append this
section with the current date to each of the elements from this query". The
strawman makes one invocation one transaction, that is, one change object
covering every document touched, and records the alternative of one change object
per document. See [Change objects](change-objects.md).

### Searching

`search` is `query` with a default first stage of body-text matching, and
frontmatter and body clauses combine as they do anywhere else. The short flags in
his search example are not interpreted by the strawman.

## What the strawman assumes

The grammar document separates what is proven from what is assumed, and marks
three readings as assumptions rather than statements of Eduardo's:

- that `# ##["Specs"][2]` reads as a heading path ending in a child index;
- that a pipeline in the style of `jq` can carry mutation verbs;
- that one grammar can serve `query`, `search`, and `write --path`.

Nothing is proven, because no code exists, which is the design phase working as
intended. No spike can run before the grammar settles.

## Still unspecified

These are the grammar document's own open items. Each is Eduardo's to settle, and
no page in this book answers any of them.

1. **One grammar or two.** Whether the three entry points share one expression
   language or whether the document-set query and the within-document path are
   separate grammars with a defined seam between them. His answer on 2026-08-31:
   "I honestly don't know."
2. **What a path can address.** Whether a path selects by heading level, by
   heading title, and by an index among children as the strawman reads it, and
   whether a path can also reach list items, paragraphs, tables, code fences,
   remargin comment blocks, and frontmatter keys. What the `[2]` counts is part of
   the question: the second child element under that heading, the second heading
   carrying that title, or something else.
3. **What a projection outputs.** Markdown fragments, frontmatter values as YAML
   or as JSON, file paths, or a mix chosen per clause — and which of those is the
   default.
4. **Mutation verbs and the transaction boundary.** Which verbs beyond appending
   the language carries, and whether one query-driven mass modification is one
   change object or one change object per document touched.
5. **How search composes with query.** Whether search is a separate verb with its
   own clause syntax or a query with body matching as its default stage, and what
   the short flags in his search example mean.
6. **Performance goals for the parser.** Eduardo will establish them; until he
   does, nothing about the parser is testable.
7. **The spelling itself.** Operators, quoting, escaping, and how a path is
   written inside a pipeline as opposed to as the `--path` flag. Nothing is
   written down beyond the four expressions above.

## What hangs on the grammar

The grammar document records three consequences, and they are why it is being
worked on before any code is written.

- **Transactions.** Every change through remargin is a change object with
  byte-identical replay and unapply, and a query-driven mass modification is such
  a change. See [Change objects](change-objects.md).
- **Shapes.** A shape can be declared for a whole document or for a sub-part of
  one, so whatever the grammar can address is what a shape can govern. See
  [Shape language](shape-language.md).
- **Permissions.** Eduardo stated on 2026-08-31 that he plans a permissions
  system in remargin where users have view or edit access to files and even to
  parts of markdown documents, so that remargin shows only the portion a person
  has access to — "this is what it's all about". A part of a document that access
  is granted to has to be addressable, and the grammar is what makes it
  addressable.

The grammar also governs the query engine and its output formats, the `query`,
`search`, and `write` verbs, and the linter. The grammar document declines to
estimate the size of the work while its open items remain.

## Where this leads

- [archie query](cli/query.md), [archie search](cli/search.md), and
  [archie write](cli/write.md) — the three commands the grammar serves.
- [Output formats](output-formats.md) — what a run prints today, and the silence
  about what a query prints.
- [Design status](../design-status.md) — the full list of open questions in the
  corpus.
