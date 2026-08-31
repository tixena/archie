# Inheritance

**Status:** Mixed — see markers.

In the vault the design corpus was written against, a folder is not just a
container: it declares things about everything below it. A folder's `index.md`
file says which properties every file underneath must carry, and which kinds of
document are allowed to live there. This page records that mechanism as the corpus
describes it, and it records the corpus's own verdict on it, which is that the
mechanism is wanted but is not agreed.

## The state of this mechanism

**Decided.** Something like inheritance is wanted; whether this is the way it
works is not settled. Eduardo, on 2026-08-16, after reading the mechanism back:
"I like it but now I'm not super convinced of this. We should do something like
this but I'm not sure this is the way. Let's discuss a bit further on other
aspects first." The root design's purpose list now carries the mechanism with the
note that it is under reconsideration and parked until other aspects are
discussed. No alternative has been proposed anywhere in the corpus.

Everything below is therefore **Proposed**: it is written down, in detail, with
worked examples, and it is not agreed.

## How it works as written

**Proposed.** The mechanism, from the workspace analysis, is four rules.

1. Every folder has an `index.md`.
2. That file's frontmatter declares an `inherits:` block: properties that apply to
   every file in the folder and in all of its subfolders.
3. Child files must carry those properties, and the validation tool enforces it.
4. Child files may add properties of their own, and may never remove an inherited
   one.

**Proposed.** Inherited properties accumulate down the tree: each level adds to
what it received. The worked example in the analysis runs three levels deep. A
folder for the Tixena area declares `area: tixena`. The folder for the Tixena team
sits below it, inherits that, and adds `role: employee`. A person's file in that
folder must therefore carry both `area: tixena` and `role: employee`, and is free
to add its own properties on top — an email address, a job title, a start date.

**Proposed.** There is no override. The corpus contains no mechanism for a child
to declare a different value for an inherited property. The example files say the
tool "validates they match", from which the corpus draws the consequence directly:
a child that declares a different value fails validation.

## The second job of the same file

**Proposed.** A folder's `index.md` also declares an `allows:` block, which is
folder rules rather than inheritance proper but rides on the same file. It names
which resource types may live in the folder, and whether the folder may have
subfolders at all. The analysis gives the pattern: a team folder allows only the
`person` type and no subfolders, while a projects folder allows both `index` and
`project` and permits subfolders, because a project can be a single file or a
folder with its own index.

The analysis lists three purposes for an `index.md` together: describing the
folder for a human reader, declaring what belongs there, and declaring what
children inherit.

## What archie does with it

**Proposed.** Three pieces of the root design act on the mechanism.

- A validation rule named `inheritance` reports a missing inherited property as an
  error, naming the property, the value expected, and the `index.md` that defined
  it, and declares that automatic repair can add the missing property.
- A rule named `index-required` reports a folder that has no `index.md`, and
  declares that repair can create one carrying the inherited properties.
- A rule named `allowed-types` reports a file whose type the folder's `allows`
  block does not permit, naming the allowed types and the file that declared them.

**Proposed.** A command prints the chain. `archie trace <FILE>` walks from the
root down to one file and shows, level by level, what each `index.md` contributes
and where it came from, ending with the properties the file must have and the
properties it does have.

```console
$ archie trace src/tixena/people/team/dylan_minaya.md
```

**Proposed.** The root design's configuration file names which properties cascade
at all, as a list holding `area`, `client`, and `role`.

## Why all of this is provisional twice over

**Decided.** Beyond Eduardo's parking of the mechanism itself, the rules that
enforce it are inside the deferral note that covers the root design's whole
validation-rule catalog: those rules, the configuration blocks behind them, and
the list of automatic repairs encode the earlier hard-coded model, and each
becomes a consequence of what the schemas say once the schema language is
designed. So the inheritance rules would be reworked even if the mechanism
survived unchanged.

**Decided.** The command that displays the chain is in a second kind of limbo.
`archie trace` appears in the root design and is absent from the list of commands
Eduardo wrote on 2026-08-30, and what becomes of the root design's
vault-application commands is one of the program's open questions. The
[archie trace](../reference/cli/trace.md) reference page records that status.

**Not in the corpus.** The `allows:` block names resource types, and resource
types moved to remargin on 2026-08-30 — see
[Shapes, schemas, and resource types](shapes-schemas-resource-types.md). Nothing
says how a folder rule that names remargin's types is expressed once archie no
longer owns the type vocabulary. This book does not answer it.

**Not in the corpus.** Whether inheritance is a folder mechanism at all under the
schema-driven direction. The mechanism as written is bound to folders and to a
file named `index.md`; shapes and schemas, as Eduardo described them on
2026-08-25, are bound to documents and to parts of documents. Nothing in the
corpus relates the two, and no page in this book assumes a relation.

## The philosophy the mechanism serves

**Proposed.** The workspace analysis states the ideas the structure exists for,
and they are worth having in one place because other rules refer to them.

- **Folder-first organization.** The folder structure represents life areas and
  organizational hierarchy, and every file takes context from its location.
- **Every file is a resource.** Every markdown file has a type, properties in
  frontmatter, and flat keyword tags, with properties carrying structured values
  and tags existing for discovery.
- **One global inbox, per-area archives.** A single inbox folder catches
  unprocessed captures, which supports automation dumping things into it, while
  each area keeps its own archive for finished or inactive content.

**Proposed.** Archiving has one trigger and no other. A resource moves to an
archive when its `archived` property is set to true, and the analysis states
explicitly that this is the only trigger, with no automation based on how old
something is. The archive mirrors the structure of the area it belongs to, and a
validation rule reports any file that claims to be archived while living outside
an archive folder.

**Proposed.** The inbox is the one place where the rules relax. Inbox files may
have no frontmatter at all, and the analysis names this as the single exception to
the expectation that every file carries metadata, because the inbox is where
unprocessed content lands. An informational rule reports inbox files that still
need processing.
