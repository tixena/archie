# archie ls

**Status:** Proposed — see markers.

`archie ls` lists the contents of a corpus. The corpus names the command and
describes no behavior for it.

## Synopsis

```console
$ archie ls
```

Eduardo Burgos wrote this line on 2026-08-30 on the planning document, first in a
list of the things he would actually run. It is the whole of what the design
corpus says about `archie ls`: a verb and no argument.

## Status of the command

**Proposed.** Eduardo stated on 2026-08-30 that the list he wrote is not written
in stone — the verbs and the syntax can change and are not a hard requirement.
That statement is recorded as decision G4 on the grammar document, which is why
this command is Proposed rather than Decided even though Eduardo named it
himself.

## Arguments and options

None is written down. The one command line has neither.

## Behavior

The corpus describes none. What is recorded, as an observation made on the
planning document when Eduardo's list was read against the documents: `ls` is the
name of an operation that already exists in remargin, as are `get`, `lint`,
`search`, `write`, `rm`, and `query`; `link` and `init` are not. Whether archie's
command surface is meant to mirror remargin's operation set is not answered
anywhere — the vision blueprint holds it as part of its question about what
"archie becomes the core of remargin" means mechanically.

Nothing states what a listing shows, whether it recurses, whether it lists only
markdown files or every file, whether it honors the ignore list in the
configuration file, or what it does when no path is given.

> **Suggestion (documentation author):** this page has nothing to describe until
> four things are decided. Eduardo would have to say: what a row of the listing
> carries — a path alone, or a path with the resource's type and other frontmatter
> values; whether a listing recurses by default; whether it lists non-markdown
> files alongside markdown ones, given that `archie get` retrieves them; and
> whether the ignore list in `archie.toml` applies. Until then this book states
> only the command line Eduardo wrote.

## Examples

```console
$ archie ls
```

Eduardo's own example. The design specifies no output for it, so none is shown,
and no second example is offered: any argument or option would be an invention.

## Exit codes

The root design document's exit-code table is written in the language of the
validation run and does not mention `archie ls`. See
[Exit codes](../exit-codes.md).

## Related pages

- [archie get](get.md) — retrieving what a listing names.
- [archie query](query.md) — selecting documents by an expression rather than by
  location.
- [archie and remargin](../../concepts/archie-and-remargin.md) — the overlap
  between archie's verbs and remargin's operations.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
