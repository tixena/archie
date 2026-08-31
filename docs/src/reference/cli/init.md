# archie init

**Status:** Mixed — see markers.

`archie init` sets up a corpus: it creates archie's configuration file and the
folder index files that are missing. It is the one command that appears both in
the root design document and in Eduardo Burgos's own command list.

## Status of the command

**Proposed.** Eduardo wrote `archie init .` on 2026-08-30 as one of the things he
would run, and stated on the same document that the list is not written in stone —
the verbs and the syntax can change. Unlike `fix`, `inbox`, `trace`, `export`,
and `import`, `init` is not one of the commands whose fate the vision blueprint
holds open: it is in Eduardo's list. Its option set, however, comes only from the
root design document, and one of the two jobs those options describe rests on a
mechanism that is parked.

## Synopsis

```console
$ archie init [PATH]
$ archie init .
```

The first line is the root design document's; the second is Eduardo's.

## Arguments

| Argument | What the corpus says |
|---|---|
| `[PATH]` | Where to initialize. Optional in the root design's spelling; Eduardo's example passes `.` explicitly. The corpus does not state what the default is when it is omitted. |

## Options

From the root design document's command section.

| Option | What the corpus says |
|---|---|
| `--config` | Create `archie.toml` if it is missing. |
| `--indexes` | Create the missing `index.md` files. |
| `--all` | Do both. |

The corpus does not say what `archie init` does when none of the three options is
given, whether an existing configuration file is left alone or refreshed, or what
the newly created files contain beyond the note that a created index file carries
the properties inherited from its parents.

## Behavior

**Root design.** Two jobs. The first is writing `archie.toml` at the repository
root, the file whose settings the [Configuration](../config.md) page documents.
The second is creating an `index.md` in every folder that has none, which is the
same repair the `index-required` validation rule offers: the root design says such
a file can be created carrying the properties it inherits from the folders above
it.

**Superseded spelling.** The workspace analysis document names a separate command,
`archie init-indexes`, for the second job. The root design folds that job into
`archie init` under the `--indexes` option, and no later document revives the
separate spelling. No page in this book uses it.

**Parked.** The mechanism the `--indexes` job serves is under reconsideration.
The root design's purpose section records Eduardo's statement of 2026-08-16:
something like property inheritance from folder index files is wanted, but
whether the `inherits:` block in a folder's `index.md` is the way it should work
is not settled, and the question is parked until other aspects are discussed. No
alternative is proposed anywhere. So the file `--indexes` creates has a purpose
whose shape may change.

For what an index file declares today — an `inherits:` block of properties that
apply to every file in the folder and its subfolders, and an `allows:` block
naming which resource types may live there and whether subfolders are permitted —
see [Inheritance](../../concepts/inheritance.md).

## Examples

```console
$ archie init .
$ archie init --all
```

The first is Eduardo's. The second is the root design's option set applied to its
own spelling. The design specifies no output for either, so none is shown.

## Exit codes

The root design document's exit-code table is written in the language of the
validation run and does not mention `archie init`. Of its entries, a
configuration error and a file system error are the two that describe conditions
an initialization run can meet; the corpus does not say so. See
[Exit codes](../exit-codes.md).

## Related pages

- [Configuration](../config.md) — the file this command creates.
- [Inheritance](../../concepts/inheritance.md) — what a folder index file
  declares, and why the mechanism is parked.
- [archie check](check.md) — the `index-required` rule that reports the folders
  this command would fill in.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
