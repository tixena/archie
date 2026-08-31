# Enforce resource types

**Status:** Mixed — see markers.

A *resource type* says what a document is: a person, a project, a task, a
contract. The type is remargin's idea and remargin's property. archie's job is to
check that documents satisfy it. This guide walks that division in the order the
work happens, and marks the one place in the middle where the corpus has no
mechanism at all.

## Where the type lives

**Decided.** Resource types belong to remargin, not to archie. Eduardo Burgos
settled it on 2026-08-30, superseding what he had said on 2026-08-16, in a thread
on the root design document's purpose list: "I must say that I thought about this
and now I want to have resource types as part of remargin, not archie. Since
resource types is too much of business logic I think that archie shouldn't have
that. It should only have 'ways to help remargin achieve this'." The requirements
document records owning resource types as a stated non-goal for archie.

**Decided.** Enforcement is archie's. The remargin feature ledger holds the type
idea in Eduardo's words from 2026-08-25 — "remargin must have a resource type
(just like we've been thinking)" and "the linter must make those items (or
resources, or markdown, whatever you want to call it) respect the linter" — and
names the enforcement engine: archie's linter with user-specified shapes, with
strictness following archie's strict and loose schema modes.

**Decided.** Two things archie's means have to allow, from Eduardo's earlier
statement of 2026-08-16 that survived the change of owner. Types are defined per
corpus rather than chosen from a fixed list, because the set written down in the
analysis of his own vault "were extracted from MY OwN files, but someone else may
as well have a whole different set of resource types". And one type has to be
able to extend another "to add more stuff to it".

For the three words and how they relate, see
[Shapes, schemas, and resource types](../concepts/shapes-schemas-resource-types.md).

## Step 1: declare the type on remargin's side

This step is remargin's and this book does not document it. What the corpus says
about it: a type carries the properties a document of that type must have, and it
may carry *actions* — Eduardo's example, from 2026-08-25, is a `task` type whose
actions are `defer`, `close`, and `assign`, performed through actions configured
on the type. He marked the actions idea as one he would like to explore rather
than a commitment. archie enforces types; nothing in the corpus says archie
executes actions.

## Step 2: express what the type requires as a shape

**Not in the corpus.** This is the step with no mechanism. archie enforces a type
by checking documents against shapes, and nothing anywhere says how a type
declared in remargin becomes a shape archie can check: no configuration key, no
file, no named interface between the two. The shape notation itself does not
exist either — see [Define a shape](define-a-shape.md) and the
[Shape language](../reference/shape-language.md) reference.

What is settled is the mode. A schema is strict, rejecting content the shape does
not declare, or loose, letting it through; there is no mode that accepts a
document by trimming it. So a type can be enforced as a floor or as a boundary,
once there is a way to write it down.

> **Suggestion (documentation author):** the gap in this step is not the shape
> notation but the handover. Even with a notation, nothing in the design says how
> remargin's type definitions reach archie's linter. Eduardo would have to decide
> which of three shapes that handover takes: remargin passes type definitions to
> the engine at call time, so archie stores none of them and a corpus has no
> archie-side record of its types; or a corpus keeps its type definitions in
> files archie reads, with remargin as one writer of those files; or the
> definitions live in archie's configuration, which is the arrangement the root
> design's superseded `[resource_types]` block assumed and the one his 2026-08-30
> statement argues against. The choice decides what a reader of this guide edits,
> and this book does not assume an answer.

## Step 3: run the linter over the corpus

```console
$ archie check src/tixena/
$ archie lint folder/
```

The first spelling is the root design document's, the second is Eduardo's. See
[Lint a vault](lint-a-vault.md) for the run and its options, and
[archie check](../reference/cli/check.md) for the option table.

## Step 4: read the type findings

**Deferred.** Three rules in the root design's catalog concern types today, and
all three are inside the deferral note Eduardo's schema direction put over that
catalog. They are recorded here because they are what the design currently says,
not because they will survive.

| Rule | What it reports |
|---|---|
| `type-required` | A file with no `type` property. An error, and one the repair run leaves alone because a person has to decide the type. |
| `type-valid` | A `type` value that is not one of the defined types. An error whose message lists the valid types. |
| `allowed-types` | A file whose type is not in the `allows.types` list declared by its folder's index file. An error naming the allowed types and the file that declared them. |

Two of those rules read a fixed list. The `type-valid` message enumerates the
fifteen types in the root design's `[resource_types] valid = [...]` block, and
that block is exactly the artifact Eduardo's 2026-08-30 statement supersedes: the
list came from his own files and no list belongs to archie. The
[Validation rules](../reference/validation-rules.md) reference reproduces each
message with that status attached.

## Extending a type

**Decided, in outline only.** A type can extend another to add to it. Nothing
states what extension does in terms of shapes: whether an extending type may
relax what it extends, whether the strict and loose modes interact across an
extension, or what happens when two extended types disagree. This book fills none
of that in.

## What the corpus does not say

- **Whether a resource type is a schema.** The corpus never states the relation in
  one place. It says a type is enforced by the linter, and it says a schema
  carries a mode; it does not say that one type is one schema.
- **How a folder rule names a type it no longer owns.** A folder's `index.md`
  declares an `allows:` block listing resource types, and types moved to remargin
  after that mechanism was written. See [Inheritance](../concepts/inheritance.md).
- **What a type violation looks like once shapes replace the catalog.** Every
  message above belongs to the fixed rules.

## Related pages

- [Shapes, schemas, and resource types](../concepts/shapes-schemas-resource-types.md)
  — the three words and the boundary between them.
- [archie and remargin](../concepts/archie-and-remargin.md) — where the line
  between the two runs, and what is still open about it.
- [Define a shape](define-a-shape.md) — the declaration side of the same work.
- [Lint a vault](lint-a-vault.md) — running the check that enforces the types.
- [Validation rules](../reference/validation-rules.md) — the rules in full, with
  their deferral.
