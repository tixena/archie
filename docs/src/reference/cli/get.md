# archie get

`archie get` retrieves a resource by its path. A resource is a folder, a markdown
file, an image, or a document in Portable Document Format: non-markdown files are
retrievable, not merely referenced.

## Synopsis

```console
$ archie get folder/
$ archie get folder/file.md
$ archie get folder/image.png
$ archie get folder/doc.pdf
```

The path is required. *Options are not yet specified.*

## Not yet specified

- What a folder retrieval returns: a listing, a concatenation, or an archive.
- What a markdown retrieval returns: the file's bytes unchanged, the parsed
  document, or the frontmatter alone.
- How binary content is delivered: written to standard output, written to a named
  file, or described rather than emitted.
- Output, and the exit code for a path that does not exist.

See [Design status](../../design-status.md) for the open questions around this
command.

## Related

- [archie ls](ls.md) — listing what can be retrieved.
- [archie query](query.md) — selecting documents by an expression.
- [Documents and schemas](../../concepts/documents-and-schemas.md) — the document
  as an abstraction independent of a file.
- [Command-line interface](index.md) — global options and the command table.
