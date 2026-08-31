# Define a shape

**Status:** Mixed — see markers.

A *shape* is your own description of what a markdown document, or one part of a
document, has to look like. archie's linter checks documents against shapes, and
the shapes belong to whoever writes them rather than to archie.

This guide walks the decisions that declaring a shape needs, in the order they
have to be made. It cannot show you a shape written out. No shape appears
anywhere in archie's design corpus: the corpus fixes what a shape is for and
settles nothing about how one is spelled. So each step below ends either in a
settled answer or in a plain statement that the answer does not exist yet. The
[Shape language](../reference/shape-language.md) reference page records the same
absence at greater length.

## Before you start

**Decided.** archie has a linter, and the shapes it checks against are specified
by the user. Eduardo Burgos stated it on 2026-08-25, and the archie feature
ledger records it in his words: "archie must have a linter. I can specify the
*shape* of any markdown (or sub-markdown part)."

**Decided.** The linter is archie's; the ideas it enforces are remargin's. The
program's vision blueprint records the boundary Eduardo drew on 2026-08-25:
archie holds the linter and the schema engine, while remargin holds the resource
types and the actions declared on them. If what you actually want to declare is
what a `person` or a `project` document must carry, start with
[Enforce resource types](enforce-resource-types.md) instead — that job begins on
remargin's side of the boundary.

## Step 1: decide what the shape governs

A shape governs either a whole markdown document or a sub-part of one — a
section, a fragment.

**Decided.** Both are in scope. The sub-part half is in Eduardo's own sentence
above, in the parenthesis "or sub-markdown part", and the vision blueprint
carries it into the boundary it records.

**Not in the corpus.** A shape for a sub-part has to name the sub-part it
governs, and no vocabulary for naming part of a document exists. That vocabulary
is the same one a path expression would need, and the grammar document lists what
a path can address among the things nobody has settled: whether a path selects by
heading level, by heading title, and by an index among children, and whether it
can reach list items, paragraphs, tables, code fences, remargin comment blocks,
and frontmatter keys. See [Query grammar](../reference/query-grammar.md).

So a whole-document shape is the only one this step can be finished for, and even
that waits on step 3.

## Step 2: choose strict or loose

A shape declares what a document must contain. The mode decides what happens to
everything the shape did not mention.

**Decided.** There are two modes. Eduardo, 2026-08-25: "Some remargin schemas can
have a *strict* or *loose* mode — think of it as Zod's `.strict()`, `.loose()` or
`.strip()`." Zod is a schema-declaration library from the TypeScript ecosystem,
and the three method names name three ways of treating undeclared content: reject
it, allow it, or accept it and silently discard it.

**Decided.** Strict rejects content that falls outside the declared shape. Loose
lets that content through. Choose strict when a document has to carry nothing but
what the shape names; choose loose when the shape is a floor rather than a
boundary.

**Decided.** There is no third mode. In his words, from the same statement: "I
don't think we will be using `.strip()` here because I wouldn't want to accept
content and trim it." A document is either accepted as it stands or reported as
failing. archie does not make a document validate by removing part of it.

**Not in the corpus.** Which mode applies when a shape declares none, and whether
the mode is a property of the shape, of the document, or of the command that runs
the check.

> **Suggestion (documentation author):** the order of the four steps on this page
> is this book's arrangement, not the design's, and one of them may be in the
> wrong place. Choosing a mode is written here as part of declaring a shape,
> which assumes the mode is stored with the shape. If instead the mode is chosen
> when the linter runs, then it is not a step in defining a shape at all but an
> option of the check, and the same shape can be enforced strictly in one corpus
> and loosely in another. Eduardo would have to decide which, because the two
> answers produce different guides and a different command-line surface.

## Step 3: bind the shape to the documents it governs

**Not in the corpus.** How a shape reaches the documents it governs is not
written down, and the corpus has named that absence since 2026-07-27, when the
open items left by Eduardo's clarifications about schemas were recorded on the
root design document: what the schema language looks like, and how schemas attach
to folders and to files.

Two mechanisms in the corpus look as though they might be the answer, and neither
is.

- A folder's `index.md` can declare which resource types are permitted inside it
  and which properties every file below it must carry. That mechanism is parked:
  Eduardo said on 2026-08-16 that he wants something like it but is not convinced
  the `inherits:` block is the way, and asked to discuss other aspects first. See
  [Inheritance](../concepts/inheritance.md).
- The configuration file carries a list of rules to enable. Eduardo said on
  2026-08-16 that he may allow "some 'linting rules'" in the configuration "and
  that's it. After that every resource in a vault must respect the lint rules."
  That turns rules on and off; it does not attach a shape to a document. See
  [Configuration](../reference/config.md).

Until this step has an answer, a shape cannot be written down at all, whatever
notation it would be written in.

## Step 4: run the linter against the shape

The command that validates a corpus is spelled `check` in the root design
document and `lint` in the command list Eduardo wrote on 2026-08-30. Both
spellings are on record and no document reconciles them.

```console
$ archie check src/tixena/
$ archie lint folder/
```

The options — the output format, the level at which the run fails, the rule
filter, and the ignore patterns — are written for the `check` spelling only. See
[Lint a vault](lint-a-vault.md) for the run itself, and
[archie check](../reference/cli/check.md) for the option table.

**Not in the corpus.** What the linter prints when a document fails a shape. Every
message the design specifies belongs to the fixed rule catalog, which predates
shapes entirely. See
[Validation rules](../reference/validation-rules.md).

## What you can do today

Nothing, in the sense of running something: archie has no binary yet, and its
design is still being written. Of the four steps above, the first two have
answers you can hold a conversation about and the last two do not. A reader who
needs to enforce document structure now is outside what this book can help with.

## Related pages

- [Shape language](../reference/shape-language.md) — what the corpus fixes about
  shapes and what it leaves open, including the one sketch of a notation this
  book offers as a Suggestion.
- [Shapes, schemas, and resource types](../concepts/shapes-schemas-resource-types.md)
  — the three words, what each means, and where the vocabulary is unsettled.
- [Documents and schemas](../concepts/documents-and-schemas.md) — the document
  and the schema as objects rather than files.
- [Enforce resource types](enforce-resource-types.md) — the same machinery seen
  from remargin's side.
- [Lint a vault](lint-a-vault.md) — running the check and reading its report.
