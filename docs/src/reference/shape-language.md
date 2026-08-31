# Shape language

**Status:** Mixed — see markers.

A *shape* is a description, written by a user, of what a markdown document or a
part of one must look like. archie's linter checks documents against shapes. This
page records what the design corpus fixes about shapes, and what it leaves for
Eduardo Burgos to settle. The corpus does not define a notation for writing a
shape down, so this page does not present one as fact.

Two of the sources quoted below carry a status of their own. The archie feature
ledger records each entry with the status `idea`, and the requirements document
lists them as candidate goals that Eduardo has not yet accepted as goals. So the
shape idea and the two schema modes are written as Proposed with the date he
stated them: he stated them, and the vision blueprint that puts them on archie's
side of the boundary records that boundary as stated intent rather than as a
design.

## What a shape is

**Proposed (stated by Eduardo on 2026-08-25).** archie has a linter, and the
shapes it checks against are specified by the user. The archie feature ledger
records it in his words, in the entry "Linter with user-specified shapes":
"archie must have a linter. I can specify the *shape* of any markdown (or
sub-markdown part)." A shape therefore governs either a whole markdown document
or a sub-part of one — a section, a fragment — and the linter is what enforces
it.

**Proposed (stated by Eduardo on 2026-08-25).** The linter belongs to archie.
The program's vision blueprint records the boundary Eduardo drew that day in its
section on the design: archie holds the linter with user-specified shapes and
the schema engine with its modes, while remargin holds the resource types and
the actions declared on them. The blueprint records that placement as stated
intent and not yet a design.

**Decided.** The ideas the linter enforces belong to remargin. He restated the
split on 2026-08-30 on the root design document, superseding what he had said on
2026-08-16: resource types are business logic and belong to remargin, and archie
holds only "ways to help remargin achieve this". The requirements document
records owning resource types as a stated non-goal for archie. The concept page
[Shapes, schemas, and resource types](../concepts/shapes-schemas-resource-types.md)
covers the split; this page covers only archie's side of it.

**Decided.** A shape can require, or not require, frontmatter. The root design's
purpose section says that markdown files validate against configurable schemas
for both frontmatter and structure, and that whether frontmatter is required at
all is ruled by those schemas. Eduardo wrote the underlying statement on
2026-07-26 in a thread on that section: "every markdown may (or may not, if the
rules don't specify it) have frontmatter and one of our main features is that we
can validate files against schemas (for both frontmatter and structure)". The
same thread records that which properties a document must carry is a consequence
of the governing schema rather than a fixed list.

## Strict and loose

**Proposed (stated by Eduardo on 2026-08-25).** A schema has two modes. His
words, recorded in the feature ledger entry "Schema modes: strict or loose":
"Some remargin schemas can have a *strict* or *loose* mode — think of it as
Zod's `.strict()`, `.loose()` or `.strip()`." Strict rejects content that falls
outside the declared shape. Loose lets that content through.

**Proposed (stated by Eduardo on 2026-08-25).** There is no stripping mode. In
his words, from the same entry: "I don't think we will be using `.strip()` here
because I wouldn't want to accept content and trim it." A document is either
accepted as it stands or reported as failing; archie does not silently remove
the parts a shape does not mention.

The corpus does not say which of the two modes applies when a shape does not
declare one, nor whether the mode is a property of the shape, of the document, or
of the command that runs the check.

## Extension

**Decided, with its owner moved.** A type can extend another type to add more to
it. Eduardo asked for this on 2026-08-16, on the root design's purpose thread:
resource types are defined per vault — "those were extracted from MY OwN files,
but someone else may as well have a whole different set of resource types" — and
"we should be able to 'extend' a resource type to add more stuff to it". On
2026-08-30 he moved resource types themselves to remargin, and the root design now
carries extension as one of the things remargin needs from archie's means, rather
than as an archie concept in its own right.

The corpus does not say what extension does to a shape: whether an extending type
may relax what it extends, whether the two modes interact across an extension, or
what happens when two extended types disagree.

## What a shape is attached to, and how

The corpus is silent here, and has been named as silent since 2026-07-27, when
the open items left by Eduardo's schema clarifications were written down on the
root design: what the schema language looks like, and how schemas attach to
folders and to files.

Two neighbouring mechanisms exist in the corpus and neither is that answer. A
folder's `index.md` can declare which resource types are permitted inside it and
which properties every file below it must carry; the mechanism is described in
the analysis of Eduardo's own vault, and he parked it on 2026-08-16 — something
like it is wanted, but he is not convinced that the `inherits:` block is the way,
and he asked to discuss other aspects first. Separately, the configuration file
carries a list of rules to enable, and Eduardo said on 2026-08-16 that "some
'linting rules'" may be allowed in the configuration "and that's it. After that
every resource in a vault must respect the lint rules". Neither of those attaches
a shape to a document. See [Configuration](config.md) and
[Inheritance](../concepts/inheritance.md).

## What the corpus does not define

- **A notation.** No document in the corpus contains a shape written out. There
  is no example of a shape declaration anywhere.
- **A vocabulary for body structure.** A shape that governs a sub-part of a
  document has to name that sub-part. Which parts of a document can be named at
  all is the same question the grammar document asks about path expressions, and
  it is open — see [Query grammar](query-grammar.md).
- **The relationship between the words "shape" and "schema".** The corpus uses
  both. Eduardo says "shape" when he speaks of what a document must look like and
  "schema" when he speaks of the modes and of what remargin declares. No document
  states whether they name one thing or two.
- **How a failing check is reported.** The rule catalog in the root design
  predates shapes and reports against fixed rules; what a shape violation looks
  like as a message is not written down. See
  [Validation rules](validation-rules.md).

> **Suggestion (documentation author):** the corpus fixes what a shape is for but
> nothing about how one is written, which leaves a reference page with nothing to
> show. As something concrete to react to rather than as a proposal with any
> standing, a shape could be a block in the configuration file that names the
> documents it governs, its mode, the frontmatter keys it requires, and the body
> structure it expects:
>
> ```toml
> [[shape]]
> name = "meeting-note"
> mode = "loose"
> applies_to = "notes/meetings/**/*.md"
> frontmatter_required = ["title", "date", "attendees"]
> body_required = ["## Agenda", "## Decisions"]
> ```
>
> Eduardo would have to decide three things before anything like this could be
> written down as design: the notation a shape lives in — a block in
> `archie.toml`, a markdown document with its own frontmatter, or a language of
> its own; how a shape is bound to the documents it governs — a path pattern, a
> resource type, a declaration inside the document, or a folder's index file; and
> the vocabulary a shape uses to describe body structure, which cannot be settled
> before the path expression is.

## Where this leads

- [Validation rules](validation-rules.md) — the rule catalog that the shape
  language is expected to replace.
- [Query grammar](query-grammar.md) — the addressing question a shape for a
  sub-part depends on.
- [Documents and schemas](../concepts/documents-and-schemas.md) — the document
  and the schema as objects rather than files.
- [Define a shape](../guides/define-a-shape.md) — the guide, which can go no
  further than this page does.
