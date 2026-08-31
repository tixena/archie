# archie inbox

**Status:** Mixed — see markers.

`archie inbox` reads the captures that have been dropped into a corpus without
being filed, asks a language model to classify each one, and proposes where each
should go, for a person to approve.

## Status of the command

**Root-design-only, fate open.** `archie inbox` appears in the root design
document and nowhere in Eduardo Burgos's command list of 2026-08-30. The vision
blueprint names this feature explicitly when it asks what becomes of archie's
features for maintaining a personal corpus: given that archie is to become the
core of remargin, whether classification with a language model lives inside
archie, inside a separate corpus-focused application built on the engine, or is
dropped, is open. Nothing in the corpus answers it.

Two further statements bear on this command. The root design's own principles say
the tool works offline for validation and uses a language model only for inbox
processing. And the direction Eduardo set on 2026-08-25 places in archie the
linter, the schema engine, the document and schema abstractions, the transactional
change model, and backend-agnostic versioning — a list that does not include
classification.

## Synopsis

```console
$ archie inbox [PATH]
```

## Arguments

| Argument | What the corpus says |
|---|---|
| `[PATH]` | The inbox path. Optional; the default comes from the configuration file, whose inbox setting names the folder. |

## Options

From the root design document's command section.

| Option | What the corpus says |
|---|---|
| `--dry-run` | Show the proposals without applying them. |
| `--auto` | Apply the proposals without asking. The root design marks this dangerous. |
| `--provider <NAME>` | The language-model provider: `anthropic` or `openai`. Default from the configuration file. |
| `--model <MODEL>` | The model to use. Default from the configuration file. |

The credential for the provider is read from the environment, from
`ANTHROPIC_API_KEY` or `OPENAI_API_KEY`. The corpus does not say which is
consulted when both are set.

## Behavior

**Root design.** The run has five steps: scan the inbox for files that have not
been processed; ask the model to classify each one; show what it proposes — a
type, an area, properties, and a destination path; let the person approve, edit,
skip, or reject; and apply what was approved.

**Root design.** An inbox file with no frontmatter is what "unprocessed" means.
The validation run reports such a file at the information level under the
`inbox-raw` rule, with the hint to run this command, and the configuration file
allows raw files in the inbox so that the missing frontmatter is not an error
there.

**Root design.** The purpose of the inbox itself, from the workspace analysis: one
global landing zone for quick captures made from a phone or by automation, for
items pulled in from an external source, and for notes written before they are
properly filed. Each area may also have an inbox of its own.

The corpus does not say what makes a proposal acceptable, what the person's edit
step can change, or what happens to a file that is rejected.

## Examples

```console
$ archie inbox
$ archie inbox src/inbox/ --dry-run
```

Both spellings appear in the corpus — the second in the workspace analysis. The
root design shows an illustration of the interactive prompt, offering approve,
edit, skip, reject, and quit for each proposal; that is a screen rather than
command output, and this page describes it rather than reproducing it, because
the design specifies command output only for the validation run.

## Exit codes

The root design document's exit-code table includes a language-model provider
error, which is the one entry written for this command. What the run returns when
every proposal is approved, when some are skipped, or when the inbox is empty, is
not stated. See [Exit codes](../exit-codes.md).

## Related pages

- [archie import](import.md) — the other route by which items reach the inbox.
- [archie check](check.md) — the `inbox-raw` rule that reports what is waiting.
- [Configuration](../config.md) — the inbox path, the provider, and the model.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
