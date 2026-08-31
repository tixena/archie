# Query and search

**Status:** Mixed — see markers.

Two verbs read a corpus. `archie query` selects documents by an expression over
their frontmatter. `archie search` matches a pattern against documents. This guide
covers choosing between them, writing the expression, and reading what comes back.

One thing has to be said before any of that, because it governs the whole page.
The expression language is not designed. Four command lines exist in the design
corpus, all written by Eduardo Burgos, and everything else about the language is
either a direction he has stated or a strawman put in a discussion document for
him to argue with. This guide shows his four lines and marks every other syntax on
the page as Proposed. The [Query grammar](../reference/query-grammar.md) reference
carries the strawman in full and the list of what nobody has settled.

## Step 1: choose the verb

**Decided.** Search reaches document bodies. Eduardo stated its scope on
2026-08-30, and it is recorded as a decision on the grammar document: search
"should be able to run on document bodies and on frontmatter if you specify it
(and both if you give clauses for both)". He named that requirement as one of his
reasons for archie writing its own markdown parser — matching frontmatter and body
separately means knowing where one ends and the other begins.

**Decided.** Query starts from frontmatter. The first thing archie is to do,
confirmed on 2026-08-16, is to walk a folder, parse each file's frontmatter, and
print the files whose frontmatter satisfies one expression.

**Proposed.** How the two relate is open. The grammar document's strawman treats
search as a query whose first stage matches body text, so that the two are one
language; the alternative recorded beside it is a separate verb with a clause
syntax of its own. Nothing decides between them.

## Step 2: write the expression

Two spellings of a query exist and no document reconciles them.

```console
$ archie query 'tags contains "rule"' .grit/patterns/
$ archie query --exp='type=issue'
```

The first is Eduardo's from 2026-08-16, with the expression and the path as
positional arguments; the second is his from 2026-08-30, with the expression as
the value of a named option and no path at all. Whether the path defaults to the
configured corpus root when it is omitted, whether several paths may be given, and
whether a path may be a glob are all unstated.

**Decided.** What the language is to be like. Eduardo, 2026-08-16 and again on
2026-08-30: "I'd like something like `jq` because it is extremely powerful and
also I'd like to be able to chain queries and be able to project almost
anything." `jq` is a command-line processor for JavaScript Object Notation (JSON)
in which small filters are joined into pipelines. So expressions chain, and a
query can name what it emits rather than always emitting whole documents.

**Decided, as a withdrawal.** An earlier proposal of named flags for common
fields — a type option, a status option, and a `--where` option carrying an
arbitrary expression — was written by an agent on the planning document and is
withdrawn in favor of the language above. It appears nowhere in this book as a
usable spelling.

**Proposed.** What a clause looks like. The grammar document's strawman offers
three forms, built out of the three kinds of expression Eduardo has written: a
frontmatter comparison, a frontmatter membership test, and a body match, combined
with `and`, `or`, and `not`. It is a first concrete proposal to argue with, and
nothing in it is settled.

## Step 3: search a path for a pattern

```console
$ archie search folder/ -gi "[0-9]?[a-z]*"
```

Eduardo's own line, from 2026-08-30, and the only spelling of `archie search`
anywhere in the corpus. Its two short flags are undefined: an agent asked on
2026-08-31 whether they were the grep-style flags for a global, case-insensitive
match, and the question was dropped when Eduardo answered the scope question
instead and said the syntax is not a hard requirement.

This guide shows no example that combines a frontmatter clause with a body clause,
even though giving both is Decided behavior, because the spelling of such a
command is exactly what the corpus does not provide.

## Step 4: read what comes back

**Not in the corpus.** No output is specified for either verb. The design writes
output down for the validation run, the inheritance trace, the inbox proposal, and
the export document, and for nothing else, so this book prints no query result.
What a match is reported as — a path, a line, a line number, a fragment, or a
whole document — is unstated for search as well.

**Proposed.** The strawman's default is one JSON value per line, with a markdown
form available when what comes out is a document fragment. What a projection
should produce is one of the grammar document's open items: markdown fragments,
frontmatter values, file paths, or a mix chosen per clause.

**Not in the corpus.** What either verb returns to the shell. The exit-code table
is written for the validation run, and whether a query that matched nothing is a
success or a distinct code is not stated. See
[Exit codes](../reference/exit-codes.md).

## Chaining and projecting

**Proposed.** In the strawman, stages join with a vertical bar in the manner of
`jq`, each stage receives the previous stage's selection, and a projection names
what comes out — a set of values, or a fragment of a document. Whether the three
entry points (`query`, `search`, and the path expression given to `archie write`)
share one grammar is open; Eduardo's answer when asked on 2026-08-31 was "I
honestly don't know."

## From selecting to changing

**Decided.** The same language drives modification, one document at a time or in
bulk. Eduardo's own example of what that is for: append a section carrying the
current date to each of the elements a query returns. That is what makes the
language reach into a document rather than stopping at the document boundary.

A modification made this way is a change object, with the replay and unapply
guarantees that model carries, and whether one bulk modification is one change
object or one per document is unsettled. Writing into part of one document has its
own guide: [Write through paths](write-through-paths.md).

## What you can do today

Nothing at a terminal: archie has no binary. What a reader can do is argue with
the strawman, which is what it was written for. Everything on this page that is
marked Proposed is a candidate for being replaced by something Eduardo prefers.

## Related pages

- [Query grammar](../reference/query-grammar.md) — the strawman in full, the four
  expressions that exist, and the seven open items.
- [archie query](../reference/cli/query.md) — the command, its two spellings, and
  what is decided about it.
- [archie search](../reference/cli/search.md) — the command and its settled scope.
- [Write through paths](write-through-paths.md) — addressing and changing part of
  one document.
- [Documents and schemas](../concepts/documents-and-schemas.md) — the parser the
  language runs on, and why it is archie's own.
- [A first session with archie](../tutorial/first-session.md) — the same steps in
  one pass, front to back.
