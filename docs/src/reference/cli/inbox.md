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
$ archie inbox src/inbox/ --dry-run
```

That spelling comes from the workspace analysis.

**Root design.** The design specifies this command's screen, so it is shown. Both
the capture below and its wording are the root design's own; the label it prints,
"AI Proposal", is what the language model proposed:

```console
$ archie inbox
Processing: src/inbox/quick_note.md

Content:
  "Fix the login timeout issue in ProDoctivity
   The session expires after 5 minutes which is too short.
   Users are complaining."

AI Proposal:
  Type: task
  Area: prodoctivity
  Filename: fix_login_timeout.md
  Priority: high
  Tags: authentication, ux
  Destination: src/prodoctivity/tasks/fix_login_timeout.md

[A]pprove  [E]dit  [S]kip  [R]eject  [Q]uit
>
```

The five letters at the bottom are the person's five choices, and the run stops
at the prompt until one is given. Nothing states what the screen looks like after
a choice is made, what the edit choice opens, or what the run prints when it
finishes.

**Root design.** A second capture, in the knowledge-patterns section of the same
document, shows the same flow proposing the observations and the relations it
found in the capture's text as well as its type and area:

```console
$ archie inbox
Processing: src/inbox/meeting_notes.md

AI Proposal:
  Type: note
  Area: tixena

  Suggested Observations:
    - [decision] Moving deployment to Kubernetes
    - [todo] Schedule follow-up with DevOps team
    - [fact] Current deployment takes 45 minutes

  Suggested Relations:
    - related_to [[Kubernetes Migration]]
    - attendees [[Dylan Minaya]] [[Melanie Hernandez]]

[A]pprove  [E]dit  [S]kip  [R]eject  [Q]uit
```

One detail in it is worth noting because the corpus contradicts itself elsewhere:
the relation type `attendees` is not in the root design's list of nine known
relation types, so the proposal shown is proposing a type the validation rule for
relation types would report as unknown. See
[Knowledge patterns](../../concepts/knowledge-patterns.md).

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
