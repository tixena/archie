# archie export

**Status:** Mixed — see markers.

`archie export` serializes a corpus so that another program can read it: the
frontmatter of every file, the knowledge patterns found in the bodies, and the
graph of links between resources.

## Status of the command

**Root-design-only, fate open.** `archie export` appears in the root design
document and nowhere in Eduardo Burgos's command list of 2026-08-30, so it falls
under the vision blueprint's open question about what becomes of archie's
features for maintaining a personal corpus.

## Synopsis

```console
$ archie export [PATH]
```

## Arguments

| Argument | What the corpus says |
|---|---|
| `[PATH]` | The path to export. Optional. The root design does not state the default. |

## Options

From the root design document's command section.

| Option | What the corpus says |
|---|---|
| `--include-content` | Include the content of each file. The root design warns that this makes the output large. |
| `--only-metadata` | Export only frontmatter, without the structure. |

The corpus does not say what happens when both options are given.

## Behavior

**Root design.** The command's stated purpose is to export the structure of a
corpus as JavaScript Object Notation (JSON), for a language-model agent or for
other tooling to consume.

**Root design.** What the export carries, from the knowledge-patterns section:
for each file, its frontmatter; the observations found in its body, which are
list items opening with a bracketed category such as a fact, a decision, a tip, a
warning, a question, or a task, each with its text and any tags; the relations
found in its body, which are typed links to other resources; and a graph of the
links between resources, in which each resource is a node and each relation is an
edge carrying its type.

The root design's illustration of an export document names its parts: a list of
files, each with its path, its frontmatter, its observations — each carrying a
category, its content, and its tags — and its relations, each carrying a type and
its targets; and beside that list a graph of nodes and of edges, each edge
carrying the resource it comes from, the resource it goes to, and its type.

The corpus does not say whether the output goes to standard output or to a file,
whether an export can be filtered to a subset of types, or how a relation target
written as a name resolves to a file.

**Bearing on this command.** Eduardo placed the document and the schema in archie
on 2026-08-25 as abstractions independent of any file, and placed version
tracking above a pluggable storage backend. Neither statement mentions export.
Whether serializing a corpus is a command or a capability of the engine that
remargin reaches through is not stated anywhere.

## Examples

```console
$ archie export
$ archie export src/tixena/ --only-metadata
```

The first is the root design's spelling with no argument; the second applies its
option set. The root design shows an illustration of an export document in the
knowledge-patterns section; this page describes what that document carries rather
than reproducing it, because the design specifies command output only for the
validation run.

## Exit codes

The root design document's exit-code table does not mention `archie export`. See
[Exit codes](../exit-codes.md).

## Related pages

- [Knowledge patterns](../../concepts/knowledge-patterns.md) — observations,
  relations, and the graph the export carries.
- [Output formats](../output-formats.md) — the formats the validation run
  produces, which are specified separately from this command.
- [Use archie from an agent](../../guides/use-from-an-agent.md) — the audience the
  root design names for this command.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
