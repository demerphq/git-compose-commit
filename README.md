# git-compose-commit

Build a clear, structured commit message from command-line options, then pass
it to `git commit`. Compose options can be mixed with Git commit options.

```sh
git-compose-commit \
  --subject 'Refresh cache after writes' \
  --text 'Keep cached values in sync after each successful write.' \
  --line 'Fixes: #123' \
  --all
```

This creates a commit with the message:

```text
Refresh cache after writes

Keep cached values in sync after each successful write.

Fixes: #123
```

## Install

From the project directory, install the executable with Cargo:

```sh
cargo install --path .
```

You need Git and a Rust toolchain. Use `git-compose-commit -h` (or
`--help=short`) for a compact synopsis, and `git-compose-commit --help` (or
`--help=long`) for the detailed option guide. Use `--version` to print the
installed version.

To inspect a composed message without creating a commit, use `--output FILE`.
It writes the message and skips `git commit`:

```sh
git-compose-commit --subject 'Preview this message' --text 'No commit is made.' --output /tmp/message.txt
```

The namespaced form is `--compose-output`. Output mode cannot be combined
with Git commit options.

## User defaults

The optional `~/.compose-commit.toml` file sets defaults for wrapping,
random choices, and bubble appearance. Use flat top-level TOML assignments;
blank lines and `#` comments are allowed. Command-line settings take
precedence. For example:

```toml
wrap_width = 68
rand_seed = 123
bubble_speaker = "cat"
bubble_style = "thought"
bubble_speaker_side = "below"
bubble_side = "center"
bubble_float = "center"
```

Every setting is optional. Without a configured bubble default or explicit
bubble clause, that choice remains random. `rand_seed` seeds any random
choices the tool makes; `--rand-seed` overrides it for one invocation.
Supported bubble values are the same as the `speaker`, `style`,
`speaker-side`, `bubble-side`, and `float` clauses accepted by `--bubble`.
Unknown settings and invalid values in the config file are errors.

## Compose a message

The subject is always the first line, wherever `--subject` appears in the
arguments. Other message sections appear in argument order, separated by
blank lines.

| Option | Output |
| --- | --- |
| `--subject TEXT` | Subject line. May be specified once. |
| `--text TEXT` | Prose paragraph. Whitespace is normalized and text wraps to the message margin. |
| `--field KEY:VALUE` | Add an aligned key/value row. Adjacent fields form one block. |
| `--note TEXT` | Add a wrapped paragraph labeled `Note:`. |
| `--warning TEXT` | Add a wrapped paragraph labeled `Warning:`. |
| `--wrap-width N` | Set the global message margin; defaults to 72 columns without a config setting (20–256). |
| `--detail TEXT` | Unwrapped text block with four spaces added before each line. |
| `--line TEXT` | One unwrapped line. Embedded newlines are rejected. |

Repeat `--text`, `--detail`, and `--line` to add sections. Values may follow
the option or use `=`:

```sh
--text 'A paragraph of prose.'
--text='A paragraph of prose.'
```

Words longer than the wrap margin are kept intact. `--wrap-width` also sets
the alignment margin for charts and bubbles, and the wrapping width for lists,
fields, callouts, and footers. Literal `--detail` and `--line` content stays
unwrapped. Git may apply its normal commit-message cleanup after the wrapper
supplies the message.

## Fields and callouts

Use repeated `--field=KEY:VALUE` options for a compact aligned block. The
value may contain colons; the first colon separates key from value. Keys and
values must both be non-empty.

```sh
git-compose-commit \
  --subject 'Document runtime settings' \
  --field 'Backend:sqlite' \
  --field 'Endpoint:https://example.test/api' \
  --field 'Retries:3'
```

This renders as:

```text
Backend : sqlite
Endpoint: https://example.test/api
Retries:  3
```

Use `--note` or `--warning` for labeled prose. Their continuation lines align
with the start of the text:

```text
Note:    Existing config files remain compatible.
Warning: This migration removes the legacy index.
```

The global `--wrap-width` option changes the default 72-column margin for
wrapped content and alignment:

```sh
git-compose-commit --wrap-width=60 --text 'Wrap this paragraph at 60 columns.'
```

## Tables

Use `--th` to define comma-separated column headings. Add rows with `--tr`, or
pass several semicolon-separated rows to `--table-rows`:

```sh
git-compose-commit \
  --subject 'Record cache status' \
  --th 'Name,State' \
  --tr 'cache,ready' \
  --tr 'worker,waiting'
```

The table section looks like this:

```text
+--------+---------+
| Name   | State   |
+--------+---------+
| cache  | ready   |
| worker | waiting |
+--------+---------+
```

For several rows at once:

```sh
--table-rows='cache,ready;worker,waiting'
```

Each row must have the same number of cells as the header. Cells are trimmed
and columns are sized to fit. These are simple delimiters, not full CSV:
quoting and escaping within a cell are not supported.

`--table-head` and `--table-row` are also accepted. The namespaced short forms
`--compose-th` and `--compose-tr`, and full forms `--compose-table-head` and
`--compose-table-row`, work as well.

## ASCII bar charts

Use `--hbar-chart` or `--vbar-chart` to add a horizontal or vertical ASCII
chart. Data items are comma-separated `label:value` pairs, and at least two
items are required. Without chart settings, pass the data list directly:

```sh
--hbar-chart='tests:10,docs:5,review:3'
```

When setting chart options, clauses are separated by semicolons and the data
list must use `data=`:

```sh
--vbar-chart='float=right;height=8;data=tests:10,docs:5,review:3'
```

`float` aligns the chart block within the message margin (72 columns by
default; see `--wrap-width`) and defaults to `center`. Vertical charts accept
`height` (default 6); horizontal charts
accept `width` (default 24). The largest value scales to the selected size.
Chart sizes range from 1 to 256. Values must be non-negative whole numbers.
Labels may not contain commas or semicolons, which are delimiters in this
syntax.

## ASCII flow charts

Use `--hflow-chart` or `--vflow-chart` to render a straight chain of labeled
boxes. The syntax follows the bar charts: a bare comma-separated label list,
or semicolon-separated settings with a required `data=` list.

```sh
--hflow-chart='float=center;data=Parse,Validate,Commit'
--vflow-chart='Input,Transform,Output'
```

These linear flow charts default to centered alignment. Labels stay on one
line and are sized to fit the widest box. At least two labels are required.

## Branching flowcharts

Use `--flow-chart` to start a graph, then define nodes and links with repeated
options. The graph is rendered as one message block and closes when the next
non-flowchart option appears or the arguments end:

```sh
git-compose-commit \
  --subject 'Validate uploads before saving' \
  --flow-chart='direction=vertical;float=center;nodesep=0.8;ranksep=1.0' \
  --flow-terminal='start:Start' \
  --flow-node='read:Read input' \
  --flow-decision='valid:Valid?' \
  --flow-node='save:Save' \
  --flow-node='reject:Report error' \
  --flow-terminal='done:Done' \
  --flow-edge='start -> read' \
  --flow-edge='read -> valid' \
  --flow-edge='valid -> save [label="yes"]' \
  --flow-edge='valid -> reject [label="no"]' \
  --flow-edge='save -> done' \
  --flow-edge='reject -> done' \
  --output /tmp/message.txt
```

Node IDs start with a letter or underscore and contain only letters, digits,
and underscores. `--flow-node` makes a box, `--flow-decision` a diamond, and
`--flow-terminal` an oval start/end node. Edge arguments use DOT-style directed
links, for example `FROM -> TO`; labels use DOT-style `[label="TEXT"]`
attributes. Unlabeled chains such as `A -> B -> C` are also accepted. The
wrapper generates DOT internally; arbitrary DOT source is not accepted. Set
`direction=vertical|horizontal`, `float=left|center|right`, and optionally
`nodesep=N` and `ranksep=N` (each greater than zero and at most 10). Direction
defaults to vertical, float defaults to center, and spacing defaults are 0.8
and 1.0. A graph needs at least two declared nodes and one edge. Nested graphs
are not supported. Branching diagrams are laid out with Graphviz, which must
be installed and available as `dot`.
In horizontal graphs, branches from one node align their turn columns. If
their labels differ, they take separate paths; edges merging into one node
share a vertical column.

## Speech and thought bubbles

Use `--bubble` to draw a phrase with an ASCII speaker avatar. The phrase may
contain spaces; separate clauses with semicolons:

```sh
--bubble='speaker=cat;phrase=Mooooo;float=center'
--bubble='speaker=stick-figure;style=thought;phrase=Maybe'
```

Choose `speaker=cat`, `speaker=dog`, or `speaker=stick-figure`. The style
may be `voice` (also `speech`) or `thought`. `speaker-side=left|right` puts the
avatar beside the bubble; `below` places it underneath. For an avatar below,
`bubble-side=left|center|right` positions the bubble relative to it. `float`
aligns the entire composition within the message and accepts `left`, `center`,
or `right`. Any omitted bubble settings are chosen randomly unless configured
in `~/.compose-commit.toml`; specify settings when you want to choose their
exact values. Phrases wrap to fit the message margin.

Use `--rand-seed=N` to control all random choices made by the tool, including
bubble defaults. The namespaced alias is `--compose-rand-seed=N`. The same seed
and arguments produce the same output; without a seed, choices vary between
runs.

## Lists

Each `--list-item` adds an item. With no active list, the first item starts a
numbered list. Use `--numlist` for a numbered list or `--bullets` for a bullet
list; equivalently, use `--list=numbered` or `--list=bullets`.

Starting a list inside an open list nests it. `--list-end` closes the current
list and returns to its parent:

```sh
git-compose-commit \
  --subject 'Refresh the cache safely' \
  --numlist \
  --list-item 'Write the new value' \
  --bullets \
  --list-item 'Keep readers on the previous generation' \
  --list-end \
  --list-item 'Advance the generation'
```

This produces:

```text
1. Write the new value
  * Keep readers on the previous generation
2. Advance the generation
```

List items wrap with continuation lines indented. You can omit `--list-end` at
the end of the arguments or before another message section.

## Numbered footers

Put a `[#N]` reference in a `--text` paragraph or `--line`, then define its
text with `--footer-N`:

```sh
git-compose-commit \
  --subject 'Validate cache updates' \
  --text 'Publish the new generation only after validation [#1].' \
  --footer-1 'Readers keep using the previous generation until then.'
```

The footer is appended to the message:

```text
Validate cache updates

Publish the new generation only after validation [#1].

[#1] Readers keep using the previous generation until then.
```

Footer options may appear before or after references. Footers are listed at
the bottom in number order, with aligned labels and wrapped text. Missing
footers, unreferenced footers, and duplicate definitions are errors.
References inside backtick-delimited code spans are treated as literal text.
Only `--text` and `--line` are scanned for references.

## Git options and paths

Other arguments are passed through to `git commit`, so you can mix them with
compose options:

```sh
git-compose-commit --all --subject 'Refresh the cache' --signoff
```

The composed message is supplied using Git's `-F` option. Do not pass Git
message options such as `-m`, `--message`, or `--file` as they conflict with
the generated message file.

`--` keeps its usual Git meaning: it ends option parsing and is passed through
to Git. Arguments after it are pathspecs:

```sh
git-compose-commit --subject 'Update the parser' -- src/parser.rs
```

## Option aliases

The short, unprefixed options are convenient to type, but could conflict with
options added by a future Git release. Use the namespaced forms in scripts or
when you want to avoid that risk.

| Concise option | Namespaced equivalent |
| --- | --- |
| `--subject` | `--compose-subject` |
| `--text` | `--compose-text` |
| `--detail` | `--compose-detail` |
| `--line` | `--compose-line` |
| `--field` | `--compose-field` |
| `--note` | `--compose-note` |
| `--warning` | `--compose-warning` |
| `--wrap-width` | `--compose-wrap-width` |
| `--th`, `--table-head` | `--compose-th`, `--compose-table-head` |
| `--tr`, `--table-row` | `--compose-tr`, `--compose-table-row` |
| `--table-rows` | `--compose-table-rows` |
| `--hbar-chart` | `--compose-hbar-chart` |
| `--vbar-chart` | `--compose-vbar-chart` |
| `--hflow-chart` | `--compose-hflow-chart` |
| `--vflow-chart` | `--compose-vflow-chart` |
| `--flow-chart` | `--compose-flow-chart` |
| `--flow-node` | `--compose-flow-node` |
| `--flow-decision` | `--compose-flow-decision` |
| `--flow-terminal` | `--compose-flow-terminal` |
| `--flow-edge` | `--compose-flow-edge` |
| `--output` | `--compose-output` |
| `--bubble` | `--compose-bubble` |
| `--rand-seed` | `--compose-rand-seed` |
| `--numlist` | `--compose-numlist` |
| `--bullets` | `--compose-bullets` |
| `--list` | `--compose-list` |
| `--list-item` | `--compose-list-item` |
| `--list-end` | `--compose-list-end` |
| `--footer-N` | `--compose-footer-N` |

## Tests

Run the suite with `cargo test`:

- `src/tests/mod.rs` has unit tests for parsing and formatting.
- `tests/end_to_end.rs` runs the executable in temporary Git repositories
  and checks the resulting commits.
- `tests/fixtures/` has readable argument-and-output examples. Each line
  before the blank separator is passed as one argument; the rest of the file
  is the exact expected commit message. Fixture lines are not shell-parsed.
- `tests/fixtures/errors/` records invalid argument combinations and their
  exact stderr output.

When adding a fixture, list it in `tests/end_to_end.rs`.
