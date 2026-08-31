# Configuration

**Status:** Mixed — see markers.

archie is configured by a file named `archie.toml`, placed at the root of the
repository it governs. This page lists what the root design document puts in that
file, key by key, with the status of each. Much of it is the same deferred layer
as the rule catalog: written before Eduardo Burgos set the schema-driven
direction, kept so that it is not lost, and expected to be reworked.

**Decided.** The file is named after the tool. The root design chose the name
`archie` — derived from "archivist", one who maintains archives — over ten other
candidates, and named `archie.toml` as the configuration file in the same
decision.

## What Eduardo has said about the configuration itself

**Decided.** The configuration is where lint rules are allowed to be adjusted,
and that is all. His words on 2026-08-16, on the planning document: "I may allow
some 'linting rules' in the config and that's it. After that every resource in a
vault must respect the lint rules." He said it while settling that archie's YAML
output is standard but opinionated.

**Decided.** That opinionated stance was amended on 2026-08-30: "Archie, as an
agnostic markdown tool, can respect others but definitely remargin wouldn't." So
respecting an existing YAML formatting is a capability of archie's own parser,
and the canonical form is what remargin's write path writes. No configuration key
for either behavior exists in the corpus.

## The file the root design specifies

**Proposed.** This is the configuration block as the root design writes it. Every
key is discussed below.

```toml
[vault]
root = "src"
ignore = [".obsidian", "assets", "templates"]

[rules]
# Which rules to enable (all enabled by default)
frontmatter_required = true
frontmatter_valid = true
type_required = true
type_valid = true
created_required = true
inheritance = true
index_required = true
allowed_types = true
archive_placement = true
orphaned_assets = true      # Can be slow on large vaults
embedded_tasks = true

[rules.embedded_tasks]
min_items_to_warn = 3       # Only warn if >= 3 checkboxes

[resource_types]
valid = [
  "index",
  "person",
  "organization",
  "project",
  "task",
  "proposal",
  "contract",
  "note",
  "snippet",
  "workaround",
  "walkthrough",
  "research",
  "reference",
  "log",
  "event"
]

[inheritance]
# Properties that cascade from parent index.md
properties = ["area", "client", "role"]

[archive]
trigger_property = "archived"
folder_name = "archive"

[inbox]
path = "inbox"
allow_raw = true            # Inbox files can have no frontmatter

[assets]
extensions = ["png", "jpg", "jpeg", "gif", "svg", "pdf", "mp3", "opus", "mp4"]

[ai]
provider = "anthropic"      # or "openai"
model = "claude-sonnet-4-20250514"
# API key from environment: ANTHROPIC_API_KEY or OPENAI_API_KEY

[output]
# Default output format
format = "text"             # text, json, markdown
color = true
```

A second block, in the section of the root design on knowledge patterns, extends
the same file:

```toml
[knowledge_patterns]
enabled = true

# Observation categories (extensible)
observation_categories = [
  "fact", "decision", "tip", "warning",
  "question", "todo", "insight", "reference"
]

# Relation types (extensible)
relation_types = [
  "depends_on", "blocks", "owned_by", "related_to",
  "part_of", "supersedes", "references", "implements", "extends"
]

# Validation levels
validate_observation_format = "warn"     # off, info, warn, error
validate_relation_targets = "warn"       # off, info, warn, error
validate_relation_types = "info"         # off, info, warn, error
```

## The keys, and what is settled about each

### `[vault]`

**Proposed.** `root` names the folder the corpus lives in, and `ignore` lists
folder names a run skips; the defaults shown skip Obsidian's own folder, the
assets folder, and the templates folder.

The name of this table predates a statement of Eduardo's. On 2026-08-30 he said
that nothing about archie will require Obsidian, and the requirements document
records that as a stated non-goal; the root design's opening sentence was rewritten
to match, calling the target a markdown corpus of which his Obsidian vault is the
first. The configuration table is still called `vault` and its default ignore list
still names Obsidian's folder. The corpus does not say whether the table gets
renamed.

### `[rules]`

**Proposed, and deferred.** One boolean per rule, all rules enabled by default,
with a nested table for the rules that take settings of their own —
`min_items_to_warn` is the only one, and it sets how many checkbox items in a note
raise the embedded-tasks warning.

This table is named in the root design's deferral note as encoding the earlier
hard-coded model. Under the schema-driven direction each of these becomes a
consequence of what the schemas say. See
[Validation rules](validation-rules.md).

### `[resource_types]`

**Superseded, and kept only so that it is not lost.** A fixed list of the types a
document may declare.

Eduardo replaced this twice. On 2026-08-16 he said the list is not fixed: the
types in the workspace analysis were extracted from his own files, another vault
defines its own, and a type must be able to extend another to add more to it. On
2026-08-31 he moved the concept itself: "I must say that I thought about this and
now I want to have resource types as part of remargin, not archie. Since resource
types is too much of business logic I think that archie shouldn't have that. It
should only have 'ways to help remargin achieve this'." The requirements document
records owning resource types as a stated non-goal for archie. The block survives
in the root design under the deferral note, and falls away when the schema
language is designed.

### `[inheritance]`

**Proposed, and parked.** `properties` lists the property names that cascade from
a parent folder's `index.md` to the files below it.

The mechanism behind it is parked. Eduardo said on 2026-08-16, after an agent
described how the `inherits:` block works, "I like it but now I'm not super
convinced of this. We should do something like this but I'm not sure this is the
way. Let's discuss a bit further on other aspects first." Nothing has replaced it.
See [Inheritance](../concepts/inheritance.md).

### `[archive]`

**Proposed.** `trigger_property` names the frontmatter property that marks a
document as archived, and `folder_name` names the folder an archived document
belongs in. Both are read by the archive-placement rule.

### `[inbox]`

**Proposed, and deferred.** `path` names the inbox folder, and `allow_raw`
permits files there to have no frontmatter. That exception is the concrete form of
the global frontmatter requirement the deferral note retires: under the
schema-driven direction, whether frontmatter is required anywhere is ruled by the
governing schema.

The command that consumes the inbox is one of the features whose fate is an open
question in the program's vision blueprint. See [archie inbox](cli/inbox.md).

### `[assets]`

**Proposed.** The file extensions a run treats as assets rather than as
documents, read by the orphaned-assets rule. Eduardo's command list of 2026-08-30
reaches for an image and a PDF with `archie get`, which makes non-markdown files
retrievable resources; the requirements document holds that as a candidate goal,
not an accepted one, and no configuration key expresses it.

### `[ai]`

**Proposed, with its feature's fate open.** The provider and model used when a
language model classifies inbox captures, with the key taken from the
environment rather than from the file. The root design states as a principle that
this is optional: core validation works offline.

### `[output]`

**Proposed.** `format` sets the default output format — text, JSON, or markdown —
and `color` turns colored output on or off. Both are overridable on the command
line, and the `NO_COLOR` environment variable also turns color off. See
[Output formats](output-formats.md).

### `[knowledge_patterns]`

**Proposed.** Turns pattern recognition on or off, extends the list of observation
categories and the list of relation types, and sets the level of each of the three
pattern rules to off, information, warning, or error. See
[Knowledge patterns](../concepts/knowledge-patterns.md).

## Precedence

**Proposed.** The root design orders the sources of a setting, highest first:

1. Command-line flags
2. Environment variables
3. `archie.toml`
4. Built-in defaults

## Environment variables

**Proposed.** The root design lists five.

| Variable | What it does |
|---|---|
| `ARCHIE_CONFIG` | Path to the configuration file |
| `ARCHIE_ROOT` | Path to the corpus root |
| `ANTHROPIC_API_KEY` | Key for classifying inbox captures with a language model |
| `OPENAI_API_KEY` | The same, for the other provider |
| `NO_COLOR` | Turns colored output off |

The command line also takes a configuration path directly, with `archie.toml` as
its default.

## Rules declared in the configuration

**Proposed.** The root design settles that archie will have a way to extend its
rules, and rejects the heavier options: not WebAssembly, not an embedded Lua
runtime, both judged more than rule extension needs. What it proposes instead is a
sequence of three stages, and only the first two touch the configuration file: a
custom-rule table naming a type, the properties that type requires, and the
message to print; and a hook table naming an external command to run as part of a
check. The third stage is an embedded scripting language, and the root design
marks it as "if needed".

The stages are a proposal in the root design's own words, not an agreement of
Eduardo's, and they predate the shape language. What a user-declared shape does to
the case for custom rules in the configuration is not discussed anywhere. See
[Shape language](shape-language.md).

## What the configuration does not cover

Four parts of the design have no configuration at all in the corpus: how a shape
or a schema is attached to the documents it governs; which storage backend holds a
corpus and its versions; anything about change objects; and anything about the
Model Context Protocol server. Eduardo said in chat on 2026-08-31 that archie
exposes such a server, just like remargin, and no design document records it yet;
see [Model Context Protocol server](mcp/index.md).
