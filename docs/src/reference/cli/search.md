# archie search

**Status:** Mixed — see markers.

`archie search` matches a pattern against documents in a corpus. Its scope — what
part of a document a match may come from — is settled. Its syntax is not.

## Synopsis

```console
$ archie search folder/ -gi "[0-9]?[a-z]*"
```

This is Eduardo Burgos's own command line, written on 2026-08-30 on the planning
document as one of the things he would run. It is the only spelling of
`archie search` anywhere in the corpus.

## Arguments and options

| Part | What the corpus says |
|---|---|
| `folder/` | A path to search under. Stated only by its presence in the example; nothing says whether it may be omitted, repeated, or given as a glob. |
| `-g`, `-i` | Undefined. An agent asked on 2026-08-31 whether these are grep-style flags meaning global and case-insensitive. Eduardo answered the scope question instead, and the flag question was dropped on the ground that the syntax is not a requirement. |
| `"[0-9]?[a-z]*"` | The pattern. Its presence makes pattern matching part of `archie search`; the corpus does not state which regular-expression dialect it is written in. |

No other option is written down anywhere. In particular, nothing states how a
frontmatter clause and a body clause are spelled, even though giving both is
Decided behavior.

## What is decided

**Decided.** Search runs over document bodies, over frontmatter when frontmatter
is specified, and over both when clauses are given for both. Eduardo stated it on
2026-08-31 in reply to the flag question on the planning document, and it is
recorded as decision G3 on the grammar document. In his words: it "should be able
to run on document bodies and on frontmatter if you specify it (and both if you
give clauses for both)."

**Decided.** This is one of the reasons archie writes its own parser. Eduardo
gave it as such in the same reply: "That is one of the reasons why I need to
write my own parser." Matching frontmatter and body separately requires knowing
where one ends and the other begins, which is what archie's own parser provides.

**Decided.** The verb and its syntax can change. Eduardo stated on 2026-08-30
that the command list he wrote is not written in stone, which is recorded as
decision G4 on the grammar document.

## What is proposed

**Proposed.** The grammar document's strawman treats `archie search` as
`archie query` with a default stage that matches body text, with frontmatter and
body clauses combining the way any other clauses do. The strawman explicitly does
not interpret the `-gi` flags. Whether search is instead a separate verb with a
clause syntax of its own is listed there as unspecified, together with what those
flags mean.

## Behavior

What the corpus states is the scope rule above and nothing more. It does not say
what a match is reported as — a file path, a line, a line number, a fragment, or
the document — nor whether matches are counted, nor whether the pattern is
matched line by line or across a whole document, nor how a match inside a fenced
code block or a remargin comment block is treated. Whether the pattern is a
regular expression or a literal string is not stated either; the one example
looks like a regular expression.

## Examples

```console
$ archie search folder/ -gi "[0-9]?[a-z]*"
```

Eduardo's own example, quoted from the planning document. The design specifies no
output for it, so none is shown, and no second example is offered: any command
line combining a frontmatter clause with a body clause would be an invention,
because that spelling is exactly what the corpus does not give.

## Exit codes

The root design document gives one exit-code table, written in the language of
the validation run. It does not say what `archie search` returns, in particular
whether finding no match is a success or a failure. See
[Exit codes](../exit-codes.md).

## Related pages

- [archie query](query.md) — the expression language and what is settled about
  it.
- [Query grammar](../query-grammar.md) — the strawman, including search as a
  default stage, and the list of what is unspecified.
- [Command-line interface](index.md) — global options, configuration, and the
  command table.
