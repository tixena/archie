# archie write

**Status:** Mixed — see markers.

`archie write` writes into the part of one document that a path expression
addresses. It is the only command in the corpus that operates on a sub-part of a
document rather than on whole documents, and the notation it uses to address that
part is the subject of its own discussion document.

## Synopsis

```console
$ archie write folder/file.md --path='# ##["Specs"][2]' --append --from-body='path-to-file.txt'
```

Eduardo Burgos's own command line, written on 2026-08-30 on the planning document
as one of the things he would run. It is the only spelling of `archie write`
anywhere in the corpus.

## Arguments and options

| Part | What the corpus says |
|---|---|
| `folder/file.md` | The document to write into. One file; nothing states whether a set of documents may be given, though modification driven by a query is Decided behavior of the query language. |
| `--path=<expr>` | A path expression addressing the part of the document to write into. Its grammar is undefined; see below. |
| `--append` | Append to the addressed part, rather than replacing it. Appending is the only mutation verb Eduardo has written down. |
| `--from-body=<file>` | Take the content to write from the named file. Nothing states whether content may instead be given inline or read from standard input. |

No other option appears in the corpus, and no default is stated for any of them.

## The path expression

**Unspecified.** The expression `# ##["Specs"][2]` is not defined anywhere.
Eduardo was asked on 2026-08-31 to spell out what each part selects, and
answered: "We need to discuss extensively about this grammar. So, as you may
understand I can't reply it in this comment. More importantly I don't know all
the answers. I only have a vague idea of what I want." He asked for a document
dedicated to the grammar, which is why the grammar document exists.

**Proposed.** The grammar document's strawman reads the expression literally, as
three things combined: a heading level written as a run of `#` characters, a
heading title written as `["Specs"]`, and an index written as `[2]`. Read that
way, the example selects the level-one heading, then the level-two heading titled
"Specs" beneath it, then the second child block under that heading. The strawman
marks this reading as an assumption about Eduardo's example rather than a
statement of his.

**Unspecified.** What else a path may address. The grammar document lists as open
whether a path can reach list items, paragraphs, tables, code fences, remargin
comment blocks, and frontmatter keys, and what `[2]` counts — the second child
element under that heading, the second heading carrying that title, or something
else.

**Unspecified.** Whether the path expression and the query expression are one
language or two. Eduardo's answer on 2026-08-31 was "I honestly don't know." The
strawman assumes one language with three entry points; the alternative recorded
beside it is a document-set query and a within-document path as two grammars
joined at a defined seam.

## What is decided

**Decided.** Addressing a sub-part of a document is the direction, not an
accident of one example. Eduardo placed the shapes-based linter in archie on
2026-08-25 with shapes declared for a whole markdown document or for a sub-part
of one, and modification driven by a query means the language reaches into a
document. The path expression is the same requirement seen from the command line.

**Decided.** Every write through remargin is a transaction. Eduardo stated the
change model on 2026-08-25: any change produces an object representing a change
to a resource, modeled the way git operates, so that a base document plus a chain
of changes applied in order gives a byte-identical latest version, and unapplying
the chain in reverse returns the original. It is possible, in his words, because
the content is linted markdown with a known shape.

**Decided.** Parts of a document are units of access. Eduardo stated on
2026-08-31, recorded in the grammar document's section on why the grammar
matters, that remargin's permission system gives users view or edit access to
files and even to parts of markdown documents, showing only the accessible
portion. Whatever a path expression can address is what such a permission can be
granted on.

**Decided.** archie can preserve the formatting of frontmatter it did not write,
while remargin's write path emits archie's canonical form. Eduardo, 2026-08-31,
amending his stricter statement of 2026-08-16.

## What is proposed

**Proposed.** The mutation verbs beyond appending. The grammar document's
strawman carries `append`, `prepend`, `set`, `remove`, and `replace`; only
appending comes from something Eduardo wrote. Which verbs the language must
carry, and whether one query-driven mass modification is one change object or one
per document touched, are listed there as unspecified.

**Withdrawn.** The field-editing commands sketched on the planning document —
setting a field, removing a field, adding to an array, removing from an array,
and a bulk edit driven by a `--where` option — were written by an agent, not by
Eduardo. Under the decided direction the query language itself drives
modification, so no page in this book revives them.

## Examples

```console
$ archie write folder/file.md --path='# ##["Specs"][2]' --append --from-body='path-to-file.txt'
```

Eduardo's own example, quoted from the planning document. The design specifies no
output for it, so none is shown, and no second example is offered: a second one
would require a path expression the corpus does not define.

## Exit codes

The root design document gives one exit-code table, written in the language of
the validation run. It does not say what `archie write` returns, in particular
what happens when the path expression addresses nothing, or when the document
does not match the shape a write assumes. See [Exit codes](../exit-codes.md).

## Related pages

- [Query grammar](../query-grammar.md) — the path expression strawman and the
  list of what is unspecified.
- [archie query](query.md) — the language that also drives modification.
- [Change objects](../change-objects.md) — the transactional model every write
  produces.
- [Shape language](../shape-language.md) — shapes declared for a document or a
  sub-part of one.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
