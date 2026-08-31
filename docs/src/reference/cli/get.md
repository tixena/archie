# archie get

**Status:** Proposed — see markers.

`archie get` retrieves a resource by its path. What makes it notable is what
Eduardo Burgos's examples reach for: a folder, a markdown file, an image, and a
document in Portable Document Format.

## Synopsis

```console
$ archie get folder/
$ archie get folder/file.md
$ archie get folder/image.png
$ archie get folder/doc.pdf
```

All four lines are Eduardo's, written on 2026-08-30 on the planning document.
They are the whole of what the design corpus says about `archie get`.

## Status of the command

**Proposed.** Eduardo stated on 2026-08-30 that his command list is not written
in stone — the verbs and the syntax can change. That is recorded as decision G4
on the grammar document.

## Arguments and options

| Part | What the corpus says |
|---|---|
| The path | A folder, a markdown file, an image, or a document in Portable Document Format. Required in all four examples. |

No option is written down.

## Behavior

The corpus describes none. What the four command lines establish, recorded as an
observation on the planning document: reaching for an image and for a document in
Portable Document Format makes non-markdown files retrievable resources. The root
design document touched non-markdown files only through its `orphaned-assets`
rule, which reports an asset that no markdown file references, and through its
configuration list of asset file extensions — images, documents in Portable
Document Format, audio, and video. Retrieving them is new with Eduardo's list.

**Not accepted as a goal.** The requirements document records "non-markdown files
(images, PDFs) retrievable as resources" among the candidate goals drawn from
Eduardo's command list, and states that a candidate becomes a goal only when
Eduardo says so. He has not.

Nothing states what `archie get` returns for a folder as opposed to a file, what
it returns for a markdown file — the raw text, the parsed document, the
frontmatter alone — or how the bytes of an image or a document in Portable
Document Format are delivered on a command line.

> **Suggestion (documentation author):** Eduardo would have to decide three
> things before this page can describe a behavior. What a folder retrieval
> returns: a listing, a concatenation, or an archive. What a markdown retrieval
> returns: the file's bytes unchanged, or the document as archie parsed it. And
> how binary content is handed over: written to standard output, written to a
> named file, or described rather than emitted. remargin's own retrieval
> operation answers the third by returning binary content as an encoded block
> alongside a description of its type and size, which is one available shape and
> not a statement about archie.

## Examples

```console
$ archie get folder/
$ archie get folder/file.md
$ archie get folder/image.png
$ archie get folder/doc.pdf
```

Eduardo's own examples. The design specifies no output for them, so none is
shown.

## Exit codes

The root design document's exit-code table is written in the language of the
validation run and does not mention `archie get`. What the command returns for a
path that does not exist is not stated, though the table's file system error is
the entry that describes such a condition. See [Exit codes](../exit-codes.md).

## Related pages

- [archie ls](ls.md) — listing what can be retrieved.
- [archie query](query.md) — selecting documents by an expression.
- [Documents and schemas](../../concepts/documents-and-schemas.md) — the document
  as an abstraction independent of a file.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
