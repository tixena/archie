# archie ls

`archie ls` lists the contents of a corpus.

## Synopsis

```console
$ archie ls
```

*Arguments and options are not yet specified.*

## Not yet specified

- What a row of the listing carries: a path alone, or a path together with the
  resource's type and other frontmatter values.
- Whether a listing recurses.
- Whether non-markdown files are listed alongside markdown ones.
- Whether the ignore list in `archie.toml` applies.
- What happens when no path is given.
- Output and exit codes.

See [Design status](../../design-status.md) for the open questions around this
command.

## Related

- [archie get](get.md) — retrieving what a listing names.
- [archie query](query.md) — selecting documents by an expression rather than by
  location.
- [archie and remargin](../../concepts/archie-and-remargin.md) — the overlap
  between archie's verbs and remargin's operations.
- [Command-line interface](index.md) — global options and the command table.
