# Documents and schemas

archie's two core objects are the markdown document and the schema. Neither is a
file. A file on disk is one way a document can be stored and one place a schema
can be written down, but the engine works on the objects, not on the bytes.

## Documents and schemas are objects, not files

The engine's core types are the markdown document as an object and the schema as
an object, neither tied to a path. Two capabilities stand on that abstraction: a
[change](changes-and-versions.md) describes an edit to a resource rather than to a
path, and [version tracking over pluggable backends](backends-and-capabilities.md)
only means something once a document can exist without a filesystem underneath it.

*The representation of a document object — how a heading, a list, a table, a code
fence, or a remargin comment block is held, and what identifies a document that
has no path — is not yet specified.*

## The frontmatter engine is the foundation

Frontmatter handling is what the rest of archie stands on. Validating reads and
checks fields; repairing reads and modifies them; classifying an unprocessed
capture writes them from scratch; tracing reads them up a chain of folders;
exporting serializes them. The smallest useful case is a query over one field:

```console
$ archie query 'tags contains "rule"' .grit/patterns/
```

This walks a directory, parses each file's frontmatter, evaluates one expression
against it, and prints the paths that match — no configuration file, no rules, no
inheritance.

archie ships as a Cargo workspace of library crates with a thin command-line crate
on top, rather than as a single program with internal modules.

*Which crates the workspace holds is not yet specified.*

## archie writes its own parser

archie parses markdown with a parser of its own, covering both halves of a
document: the YAML of the frontmatter and the markdown of the body. General-
purpose crates are reference material rather than dependencies.

The parser has to serve searches that run over document bodies, over frontmatter
when frontmatter is specified, and over both when clauses are given for both — one
of the reasons the parser is archie's own code.

*Performance goals for the parser are not yet specified, and this book makes no
claim about how fast anything is.*

## Whose formatting wins

archie may respect the formatting it finds; remargin does not.

The two halves land in different places. Writing a canonical form — a fixed key
order, no preserved comments, one spelling for everything — is remargin's write
path. Reading a document written in someone else's style and giving it back in
that style is a capability of archie's own parser, available precisely because the
parser is archie's own code rather than a general-purpose YAML library. Within a
corpus that remargin governs, every resource still has to satisfy the lint rules
configuration allows.

*The canonical form — key order, quoting, indentation, how a multi-line value is
written — and the list of lint rules configuration may allow are not yet
specified.*

## What schemas cover

Schemas rule both halves of a document: frontmatter and structure. What a document
is required to carry follows from the governing schema rather than from a fixed
rule — whether frontmatter is required at all, and which properties are required,
are consequences of the schema, not constants.

*The schema language is not yet specified: how a schema is written, how it
attaches to a document or to a folder, and what it can say about structure as
opposed to frontmatter.* The vocabulary — how a schema relates to a shape and to a
resource type — is taken up on
[Shapes, schemas, and resource types](shapes-schemas-resource-types.md).

## Documents that are not markdown

Images and documents in Portable Document Format are resources archie can hand
back:

```console
$ archie get folder/image.png
$ archie get folder/doc.pdf
```

*What archie does with a non-markdown file beyond returning it is not yet
specified.*

## Related

- [archie get](../reference/cli/get.md) — retrieving a resource.
- [archie query](../reference/cli/query.md) — querying frontmatter.
- [Shape language](../reference/shape-language.md) — writing a shape down.
- [Design status](../design-status.md) — the open questions here.
