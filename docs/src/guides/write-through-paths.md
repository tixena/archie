# Write through paths

**Status:** Mixed — see markers.

`archie write` changes part of one document. A path expression says which part,
and the write applies to exactly that part rather than to the file as a whole.
This guide walks a write from addressing to committing, and it is honest about
where the addressing notation stands: it is one example Eduardo Burgos typed, and
it is not defined anywhere.

## The command in full

```console
$ archie write folder/file.md --path='# ##["Specs"][2]' --append --from-body='path-to-file.txt'
```

That line is Eduardo's, written on 2026-08-30 on the planning document as one of
the things he would run. It is the only spelling of `archie write` in the design
corpus, and the three steps below take it apart.

## Step 1: address the part

**Unspecified.** The expression `# ##["Specs"][2]` is not defined. Eduardo was
asked on 2026-08-31 to say what each piece of it selects and answered: "We need to
discuss extensively about this grammar. So, as you may understand I can't reply it
in this comment. More importantly I don't know all the answers. I only have a
vague idea of what I want." He asked for a document dedicated to the grammar, and
that document exists.

**Proposed.** The grammar document's strawman reads the expression literally, as
three things joined: a heading level written as a run of number signs, a heading
title written in brackets and quotes, and an index written in brackets. Read that
way, the example selects the level-one heading, then the level-two heading titled
"Specs" beneath it, then the second child block under that heading. The strawman
marks the reading as an assumption about Eduardo's example rather than a statement
of his.

**Unspecified.** What else a path may reach. Whether it can address list items,
paragraphs, tables, code fences, remargin comment blocks, and frontmatter keys is
open, and so is what the index counts — the second child element under that
heading, the second heading carrying that title, or something else. See
[Query grammar](../reference/query-grammar.md).

## Step 2: choose the mutation

**Decided.** Appending is on record. `--append` is in Eduardo's command line, and
appending is also the verb in his description of a bulk change: append a section
carrying the current date to each of the elements a query returns.

**Proposed.** Everything else. The strawman carries five verbs — appending,
prepending, setting, removing, and replacing — and only the first comes from
something Eduardo wrote. Which verbs the language must carry is one of the grammar
document's open items.

**Withdrawn.** The field-editing commands sketched on the planning document — set
a property, remove a property, add to an array, and a bulk edit driven by a
`--where` option — were written by an agent, not by Eduardo, and the decided
direction replaced them: the query language itself drives modification. No page in
this book uses them.

## Step 3: supply the content

`--from-body=<file>` takes the content to write from a named file. Nothing states
whether content may instead be given inline on the command line or read from
standard input, and no document in the corpus mentions standard input as an
interface to any archie command.

## Step 4: understand what the write commits

**Decided.** Every write through remargin is a transaction, and the unit of that
transaction is a change object. Eduardo stated the model on 2026-08-25: any change
produces an object representing a change to a resource, modeled the way git
operates, so that a base document plus a chain of changes applied in order gives a
byte-identical latest version, and unapplying the chain in reverse returns the
original.

**Decided.** The reason the guarantee is reachable is stated with it: the content
is linted markdown with a known shape. A change to arbitrary text can only be
recorded as a difference between two texts; a change to a document whose structure
is known can be recorded in terms of that structure. So the write path depends on
the linter and on shapes — see [Define a shape](define-a-shape.md).

**Open in the corpus.** Where a transaction ends when one expression changes many
documents: one change object covering every document touched, or one per document.
The grammar document lists it as unsettled, and Eduardo's own bulk example is
exactly the case that needs the answer. See
[Change objects](../reference/change-objects.md).

> **Suggestion (documentation author):** every other operation in the corpus that
> writes has a way to see what it would do before it does it — the repair run and
> the import have a dry run, the inbox flow shows each proposal for approval, the
> structural operations name a dry run among the things to design, and the
> reference implementation behind `archie link` computes and reports its edits
> without writing. `archie write` has none written down. Eduardo would have to
> decide whether a write is previewed before it commits, and if so whether the
> preview is an option on the command or a separate call that returns the change
> object the write would produce. remargin answers this for itself with a
> projection step that reports what an operation would do without touching disk;
> that is a fact about remargin, not a statement about archie.

## Why the addressing matters beyond this command

**Decided.** Parts of a document are units of access. Eduardo stated on 2026-08-31
that remargin is to have a permission system in which people hold view or edit
access to files "and they can even have access to parts of markdowns", so that
remargin shows a reader only the portion they may see — and, in the same thread,
"this is what it's all about". Whatever a path expression can address is what such
a permission can be attached to.

**Decided.** Shapes govern sub-parts too, so the vocabulary a path uses is the
vocabulary a shape needs. That is why the grammar is being worked out before any
code is written.

## What is not stated

- What happens when the path addresses nothing.
- What happens when the document does not have the structure the path assumes.
- Whether more than one document can be written in one invocation, even though
  modification driven by a query is decided behavior.
- What the command prints, and what it returns to the shell.

## Related pages

- [archie write](../reference/cli/write.md) — the command, its arguments, and the
  full record of what is undefined.
- [Query grammar](../reference/query-grammar.md) — the path expression strawman
  and the open items.
- [Change objects](../reference/change-objects.md) — the transactional model this
  write produces.
- [Changes and versions](../concepts/changes-and-versions.md) — the concept
  behind it.
- [Query and search](query-and-search.md) — selecting the documents a change
  applies to.
