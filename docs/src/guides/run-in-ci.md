# Run archie in continuous integration

**Status:** Mixed — see markers.

Continuous integration is the practice of running checks automatically on every
change to a repository, so that a break is caught by a machine rather than by a
person later. archie's design has this use in mind: a validation run prints a
machine-readable report and returns a process exit code that decides whether the
build fails. This guide covers the run, the code, the report, and the two places
where the design names something it does not provide.

## The principle behind it

**Proposed.** The root design document lists five design principles, and the fifth
is that archie is friendly to continuous integration: exit codes and output in
JavaScript Object Notation (JSON) for automation. That principle is what the rest
of this page rests on, and like the rest of the root design's validation layer it
is written down rather than agreed.

## Step 1: run the validation and capture the report

```console
$ archie check --format json > report.json
```

That command line is the root design document's own, written in its example block
under the label of output for continuous integration. The validation command also
appears in Eduardo Burgos's 2026-08-30 command list under the name `lint`; no
option is recorded for that spelling, so this page uses `check`, which is what the
options are written for. See [archie check](../reference/cli/check.md).

Three formats exist: text for a person at a terminal, JSON for another program,
and markdown for a report to commit or to paste into a review. A build step wants
the second, and a step that publishes a report may want the third.

## Step 2: choose what makes the build fail

```console
$ archie check --format json --fail-on warning > report.json
```

`--fail-on` takes `error`, `warning`, or `any`, and defaults to `error`. It
changes only the exit status; the findings printed are the same either way. See
[Lint a vault](lint-a-vault.md) for reading them.

## Step 3: gate on the exit code

**Proposed.** The root design gives one exit-code table for the program.

| Code | Meaning |
|---|---|
| 0 | Success, no issues. |
| 1 | Validation errors found. |
| 2 | Warnings only, when the run was set to fail on warnings. |
| 3 | Configuration error. |
| 4 | File system error. |
| 5 | Language-model provider error. |

Three things a build step should know about that table. Code 2 arises only when
the failure level is set to warnings, so a job that leaves the default in place
never sees it. Code 5 cannot come from a validation run, which uses no language
model; it belongs to the command that classifies inbox captures. And the table is
written in the language of the validation run and is not restated for any other
command, so a job that runs anything else is gating on an unspecified code. See
[Exit codes](../reference/exit-codes.md).

## Step 4: consume the report

The JSON object carries one run: the tool version, the time of the run, the corpus
root, the count of files and of assets scanned, the findings grouped into errors,
warnings, and information, and a summary holding the count at each level and the
number of files that passed. Each finding names the rule that produced it, the
path it concerns, its message, and whether it can be repaired. The object is shown
in full on [Output formats](../reference/output-formats.md).

One inconsistency inside the corpus is worth knowing before a build step reads the
repairable flag: the sample finding in the root design carries that flag set to
true for a missing frontmatter block, while the repair command's own list of what
it can fix does not include creating frontmatter. The corpus does not resolve it.

**Not in the corpus.** Whether the text output is a contract another program may
parse, or a rendering that may change. A build step should read the JSON.

## Step 5: scope the run to what matters

```console
$ archie check src/tixena/ --rule frontmatter --rule type
$ archie check --ignore templates
```

A path argument limits the run to part of the corpus, `--rule` limits it to named
rules, and `--ignore` adds patterns to skip on top of the configuration file's
ignore list. Rules that should always be off belong in the configuration file
rather than on the command line — see [Configuration](../reference/config.md).

## What the design names but does not provide

The root design's implementation plan lists three things for automated use as work
to be done, and the corpus contains none of them: an example workflow for GitHub
Actions, documentation for a pre-commit hook, and shell completions. Naming them
is all the corpus does.

> **Suggestion (documentation author):** a reader who came here for a workflow
> file needs something to start from, so here is one — this book's invention, with
> no standing in the design.
>
> ```yaml
> name: archie
> on: [push, pull_request]
> jobs:
>   check:
>     runs-on: ubuntu-latest
>     steps:
>       - uses: actions/checkout@v4
>       - name: Install archie
>         run: "# no published distribution exists yet"
>       - name: Validate the corpus
>         run: archie check --format json --fail-on warning > archie-report.json
>       - uses: actions/upload-artifact@v4
>         if: always()
>         with:
>           name: archie-report
>           path: archie-report.json
> ```
>
> The middle step is the real gap, and it is why this is a Suggestion rather than
> a guide. Nothing in the corpus says how archie is obtained: no published
> package, no release artifact, no container image, no installation instruction of
> any kind. Eduardo would have to decide how a build machine gets the binary
> before any workflow file can be more than a sketch, and separately whether
> archie ships such a workflow, a pre-commit hook, and shell completions at all,
> given that the features they serve are among those whose fate the program's
> vision blueprint holds open.

## One neighbouring behavior on record

The reference implementation behind `archie link` — the `link` subcommand of
Eduardo's `aa_tool` project — has a mode that computes and reports its edits
without writing and exits with a non-zero status when edits are pending, so that
an automated check fails while nothing is changed. That is a fact about `aa_tool`.
Whether archie's `link` inherits the convention is not stated. See
[archie link](../reference/cli/link.md) and [Link resources](link-resources.md).

## What is not decided

- Exit codes for the commands Eduardo named: listing, retrieving, linting,
  linking, initializing, searching, writing, removing, and querying have none
  attached anywhere.
- What a query that matched nothing returns, which decides whether a query can be
  used as a test in a script without inspecting its output.
- What a refused transaction returns. A structural operation moves nothing at all
  if any entry is refused, and the code it returns is not written down.
- Whether a repair run has codes of its own.

## Related pages

- [Exit codes](../reference/exit-codes.md) — the table and everything unstated
  about it.
- [Output formats](../reference/output-formats.md) — the JSON object and the other
  two formats in full.
- [Lint a vault](lint-a-vault.md) — the same run at a terminal.
- [archie check](../reference/cli/check.md) — the option table.
- [Configuration](../reference/config.md) — settings that hold for every run.
