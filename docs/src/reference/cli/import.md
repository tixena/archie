# archie import

**Status:** Mixed — see markers.

`archie import` pulls items in from a source outside the corpus and writes them
into it. Google Tasks is the one source the design works out.

## Status of the command

**Root-design-only, fate open.** `archie import` appears in the root design
document and nowhere in Eduardo Burgos's command list of 2026-08-30. The vision
blueprint names importing from Google Tasks explicitly when it asks what becomes
of archie's features for maintaining a personal corpus: whether they live inside
archie, inside a separate corpus-focused application built on the engine, or are
dropped, is open. Nothing in the corpus answers it.

## Synopsis

```console
$ archie import <SOURCE>
$ archie import google-tasks
```

## Arguments

| Argument | What the corpus says |
|---|---|
| `<SOURCE>` | The external source to import from. Required. `google-tasks` is the only value the corpus names. |

## Options

From the root design document's command section.

| Option | What the corpus says |
|---|---|
| `--list <ID>` | A specific task list to import, named by its identifier. |
| `--dry-run` | Show what would be imported without writing anything. |
| `--target <PATH>` | Where imported items are written. Default: the inbox folder. |

## Behavior

**Root design.** Items are pulled from the source and written into the target
folder, which by default is the inbox — so an import leaves work for
[`archie inbox`](inbox.md) to classify and file, rather than filing it directly.

The corpus does not say what an imported item looks like once written: whether it
carries frontmatter, which properties, or how a source item maps onto a file
name.

**Open in the corpus.** How authentication against Google Tasks is handled. The
root design lists it as one of its two open questions, and asks whether there
should be a separate command for signing in and where the resulting credentials
would be stored. No answer is recorded, and no environment variable in the
configuration section covers it.

**Open in the corpus.** Whether other sources are planned. The root design's own
phrasing is "external sources", plural, with Google Tasks as the worked example,
but no other source is named anywhere.

## Examples

```console
$ archie import google-tasks
$ archie import google-tasks --dry-run --target inbox/
```

The first is the root design's own example; the second applies its option set.
The design specifies no output for either, so none is shown.

## Exit codes

The root design document's exit-code table does not mention `archie import`. In
particular, nothing states which code an authentication failure or an
unreachable source returns; the table's entry for a language-model provider error
does not cover them. See [Exit codes](../exit-codes.md).

## Related pages

- [archie inbox](inbox.md) — what processes the items an import leaves behind.
- [Configuration](../config.md) — the inbox path that supplies the default
  target.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
