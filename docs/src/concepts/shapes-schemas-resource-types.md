# Shapes, schemas, and resource types

**Status:** Mixed — see markers.

Three words in the design corpus describe three different things, and they are
easy to confuse because they arrived at different times from different
conversations. A **shape** is what a user declares a document or a part of a
document must look like. A **schema** is the object archie's engine holds and
enforces, with a mode that decides how it treats content it did not declare. A
**resource type** is remargin's idea of what a document *is* — a person, a
project, a task — and it is enforced by archie's linter without archie owning it.

This page states what the corpus says about each, how they relate, and where the
vocabulary is not settled.

## Shape

**Proposed (stated by Eduardo on 2026-08-25).** In his words: "archie must have
a linter. I can specify the *shape* of any markdown (or sub-markdown part)." Two
things are in that sentence. Shapes are declared by the user rather than built
into archie. And a shape applies either to a whole markdown document or to a
sub-part of one — a section, a fragment — not only to a document as a unit.

The archie feature ledger records it with the status `idea`, and the requirements
document lists it among the candidate goals awaiting acceptance, which is why this
is marked Proposed. The linter itself is placed in archie by the 2026-08-25
boundary the vision blueprint records, and the remargin feature ledger names it as
the enforcement engine behind remargin's resource types.

The sub-part half of the idea has a consequence recorded elsewhere: whatever the
path expression in archie's grammar can address is what a shape can be attached
to. That makes the shape idea and the grammar mutually dependent, and the grammar
is not designed.

**Not in the corpus.** How a shape is written. No syntax, no file format, no
statement about where a shape lives or how it is bound to the documents it
governs. This book invents none of it; the
[Shape language](../reference/shape-language.md) reference page records the same
absence.

## Schema

**Proposed (stated by Eduardo on 2026-08-25).** A schema has a mode. In his
words: "Some remargin schemas can have a *strict* or *loose* mode — think of it
as Zod's `.strict()`, `.loose()` or `.strip()`." Zod is a schema-declaration
library from the TypeScript ecosystem, and the three method names he borrowed
name three ways of treating content that a schema did not declare: reject it,
allow it through, or accept it and silently discard it.

**Proposed (stated by Eduardo on 2026-08-25).** archie takes the first two and
rejects the third. Strict rejects content outside the declared shape; loose lets
it through. Stripping is explicitly unwanted, in his words: "I don't think we
will be using `.strip()` here because I wouldn't want to accept content and trim
it." No page in this book describes a mode that removes content in order to make
a document validate.

**Proposed (stated by Eduardo on 2026-08-25).** The schema engine is archie's,
while Eduardo calls the schemas themselves "remargin schemas". The vision
blueprint keeps that distinction visible deliberately: he speaks of remargin's
schemas and remargin's resource types while placing the linter and the schema
engine in archie, so the user-facing concepts are remargin's and the engine
underneath them is archie's. The blueprint records that whole placement as
stated intent and not yet a design, which is why it is Proposed here.

**Decided.** Schemas cover both frontmatter and document structure, and what a
document is required to carry follows from the governing schema rather than from a
fixed rule. That is the schema-driven direction recorded in the root design's
purpose list on 2026-07-26, and it is the reason the older rule catalog is under a
deferral note — see [Documents and schemas](documents-and-schemas.md).

## Resource type

**Decided.** A resource type is what a document is: the workspace analysis lists
`index`, `person`, `organization`, `project`, `task`, `proposal`, `contract`,
`note`, `snippet`, `workaround`, `walkthrough`, `research`, `reference`, `log`,
and `event`, with `asset` for non-markdown files that are referenced rather than
typed. Each type carries required and optional properties — a `person` may carry a
role, an email address, a phone number, a company, and a title; a `project` may
carry a client, a start date, an end date, and a repository.

**Decided.** That list is not archie's list, and no list is. Eduardo said on
2026-08-16 that the set was extracted from his own files and another vault would
define a different one, and that one type has to be able to extend another to add
more to it. On 2026-08-30 he moved ownership: resource types are business logic
and belong to remargin, and archie holds only "ways to help remargin achieve
this". The requirements document records owning resource types as a stated
non-goal for archie.

**Proposed (stated by Eduardo on 2026-08-25).** Resource types carry actions,
and those are remargin's too. In his words: "I want remargin resource types to
have *actions* associated to it" — his example being a `task` type whose actions
are `defer`, `close`, and `assign`, performed through actions configured on the
type. He marked the wider ambition as an idea he would like to explore rather
than a commitment: knowing the shape of actions would let a user interface
render any resource together with its actions, and would let agents execute
them.

## How the three relate

Putting the corpus's own sentences side by side gives the chain: remargin declares
a resource type; the type is expressed as a schema, in a strict or a loose mode;
the schema is built from shapes the user specifies for a document or for parts of
one; archie's linter checks documents against those shapes and reports what does
not satisfy them. The remargin feature ledger states the enforcement half of that
directly — resource types are enforced by archie's linter with user-specified
shapes, and strictness follows archie's schema-mode entry.

Two of the links in that chain are only as firm as the sentences above. The corpus
never says in one place that a resource type *is* a schema, and it never says how
a type extends another type in terms of schemas or shapes. Those are the gaps in
the vocabulary rather than in the mechanism.

> **Suggestion (documentation author):** Eduardo would have to decide whether
> "shape" and "schema" name one object or two. The corpus uses both words, from
> the same conversation on 2026-08-25, without ever relating them: a shape is what
> the user specifies and what the linter checks against, and a schema is what
> carries the strict or loose mode and what governs which properties a document
> must have. Three readings are available and the corpus rules out none of them:
> one object under two names; a schema as a named collection of shapes plus a
> mode; or a shape as the part of a schema that governs a sub-part of a document.
> This book uses the two words as the
> corpus uses them and asserts no relation between them; every page that needs the
> distinction says so.

## What the linter enforces today, and why that is provisional

**Proposed.** The root design carries a catalog of validation rules, grouped as
frontmatter rules, property rules, structure rules, an archive-placement rule, an
orphaned-asset rule, a rule about tasks embedded in notes, and an informational
rule about unprocessed inbox captures. Each rule has a defined
message and a severity, and some declare that automatic repair can fix them.

**Decided.** That catalog is provisional and the corpus says so. A deferral note
at the top of it records that the rules, the rule and resource-type blocks in the
configuration section, and the list of automatic repairs all still encode the
earlier hard-coded model — a global rule requiring frontmatter, `type` and
`created` as fixed requirements, a fixed resource-type list — and that each of
them becomes a consequence of what the schemas say once the schema language is
designed. The full catalog is reproduced on the
[Validation rules](../reference/validation-rules.md) reference page with that
status attached.

Two artifacts in the root design are named as falling away with it: the
`[resource_types] valid = [...]` block in the configuration and the `ResourceType`
enumeration in the type sketch, both of which encode a fixed set of types that
neither archie nor remargin is meant to have.
