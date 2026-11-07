# git-compose-commit

Build a clear, structured commit message from command-line options, then pass
it to `git commit`.  Alongside wrapped prose, lists, and tables, it can draw
charts and crucially talking ASCII animals for that whimsical flourish a good
commit needs.  Compose options can be mixed with Git commit options.

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

You need Git and a Rust toolchain. `--flow-chart` also needs Graphviz, with
its `dot` command on your `PATH` (for example, `brew install graphviz`). Use
`git-compose-commit -h` (or `--help=short`) for a compact synopsis, and
`git-compose-commit --help`, `--help=long`, or `--man` for the detailed guide
with examples and their output. Use `--version` to print the installed version.

To focus on one subject, use `--help=TOPIC`. Available topics are `message`,
`output`, `tables`, `lists`, `footers`, `charts`, `flowcharts`, `bubbles`, and
`examples`. For instance, `--help=tables` shows the table options and table
example, while `--help=examples` gathers commands with their output. Use `-h`
to see the quick synopsis and topic list.

For an assistant or a script composing a message, a useful workflow is to
read the relevant topic, choose the sections you need, and preview with
`--stdout`. Check the result, then run the command without `--stdout` to
create the commit. Set bubble options explicitly when you need repeatable
output. Examples below assume the default configuration unless shown otherwise.

## Preview and commit

By default, `git-compose-commit` builds the message and runs `git commit`.
To inspect the message first, use `--stdout`:

```sh
git-compose-commit --subject 'Preview this message' --text 'No commit is made.' --stdout
```

The preview is printed; Git is not invoked and no commit is created. Once
you are happy with it, run your command again without `--stdout` to commit.

Use `--output FILE` to write the message to a file instead. **This also skips
`git commit`**: it does not both write the file and create a commit.

```sh
git-compose-commit --subject 'Preview this message' \
  --text 'No commit is made.' --output /tmp/message.txt
```

`--output` overwrites an existing file. Neither output mode can be combined
with Git commit options such as `--all` or `--signoff`, and the two output
modes cannot be combined with each other.

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

Every setting is optional. The speaker sits below the bubble by default.
Without a configured bubble default or explicit bubble clause, other choices
remain random. `rand_seed` seeds any random choices the tool makes;
`--rand-seed` overrides it for one invocation.
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

`--detail` keeps line breaks and indents every line; `--line` keeps one line
unwrapped. For example:

```sh
git-compose-commit --subject 'Document runtime settings' \
  --text 'Keep the current defaults.' \
  --detail 'BACKEND=sqlite
RETRIES=3' \
  --line 'Reviewed-by: Ada' --stdout
```

Output:

```text
Document runtime settings

Keep the current defaults.

    BACKEND=sqlite
    RETRIES=3

Reviewed-by: Ada
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
  --field 'Retries:3' --stdout
```

This renders as:

```text
Document runtime settings

Backend : sqlite
Endpoint: https://example.test/api
Retries : 3
```

Use `--note` or `--warning` for labeled prose. Their continuation lines align
with the start of the text:

```sh
git-compose-commit --subject 'Migration note' \
  --note 'Existing config files remain compatible.' \
  --warning 'The legacy index is removed.' --stdout
```

Output:

```text
Migration note

Note: Existing config files remain compatible.

Warning: The legacy index is removed.
```

The global `--wrap-width` option changes the default 72-column margin for
wrapped content and alignment:

```sh
git-compose-commit --subject 'Wrap this paragraph' --wrap-width=30 \
  --text 'This deliberately long sentence will wrap at the selected message margin.' \
  --stdout
```

Output:

```text
Wrap this paragraph

This deliberately long
sentence will wrap at the
selected message margin.
```

## Tables

Use `--table-head` to define comma-separated column headings. Add rows with
`--table-row`, or
pass several semicolon-separated rows to `--table-rows`:

```sh
git-compose-commit \
  --subject 'Record cache status' \
  --table-head 'Name,State' \
  --table-row 'cache,ready' \
  --table-row 'worker,waiting' --stdout
```

The table section looks like this:

```text
Record cache status

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

`--th` and `--tr` are shorter aliases for `--table-head` and `--table-row`.

## ASCII bar charts

Use `--hbar-chart` for bars extending to the right. Each data item is a
`label:value` pair; `width=10` sets the longest bar to ten `#` characters:

```sh
git-compose-commit --subject 'Test coverage' \
  --hbar-chart 'float=left;width=10;data=tests:10,docs:5,review:3' \
  --stdout
```

Output:

```text
Test coverage

tests  | ########## 10
docs   | ##### 5
review | ### 3
```

Use `--vbar-chart` for upright bars. `height=5` sets the tallest bar to five
rows:

```sh
git-compose-commit --subject 'Test coverage' \
  --vbar-chart 'float=left;height=5;data=tests:10,docs:5' --stdout
```

Output:

```text
Test coverage

10 |   #
   |   #
   |   #     #
   |   #     #
   |   #     #
 0 +------------
     tests docs
```

For either chart, use at least two comma-separated items with non-negative
whole-number values. Pass a bare list such as
`--hbar-chart='tests:10,docs:5'` when no settings are needed. With settings,
separate clauses with semicolons and put the list after `data=`. `float` places
the chart at the `left`, `center` (default), or `right` of the message margin.
The default horizontal width is 24; the default vertical height is 6. Both
sizes accept 1–256. Labels cannot contain commas or semicolons.

## ASCII flow charts

Use `--hflow-chart` for a straight chain of boxes from left to right:

```sh
git-compose-commit --subject 'Release flow' \
  --hflow-chart 'float=left;data=Parse,Validate,Commit' --stdout
```

Output:

```text
Release flow

+----------+     +----------+     +----------+
|  Parse   | --> | Validate | --> |  Commit  |
+----------+     +----------+     +----------+
```

Use `--vflow-chart` to stack the boxes:

```sh
git-compose-commit --subject 'Release flow' \
  --vflow-chart 'float=left;data=Input,Transform,Output' --stdout
```

Output:

```text
Release flow

+-----------+
|   Input   |
+-----------+
      |
      v
+-----------+
| Transform |
+-----------+
      |
      v
+-----------+
|  Output   |
+-----------+
```

Both options need at least two comma-separated labels. Pass labels directly
when no settings are needed, as in `--vflow-chart='Input,Output'`. With
settings, separate clauses with semicolons and use `data=` for the labels.
`float=left|center|right` places the diagram in the message margin; the
default is `center`. Labels stay on one line and boxes share a width.

## Branching flowcharts

When the path splits, use `--flow-chart`. Give each shape a short ID, then
connect the IDs with `--flow-edge`. This example sends valid input one way and
invalid input another:

```sh
git-compose-commit \
  --subject 'Choose a path' \
  --flow-chart 'direction=vertical;float=left' \
  --flow-terminal='start:Start' \
  --flow-decision='valid:Valid?' \
  --flow-node='save:Save' \
  --flow-node='reject:Reject' \
  --flow-edge='start -> valid' \
  --flow-edge='valid -> save [label="yes"]' \
  --flow-edge='valid -> reject [label="no"]' \
  --stdout
```

Output:

```text
Choose a path

         (-------)
         ( Start )
         (-------)
             |
             |
             |
             |
             v
        /--------\
        | Valid? |
        \--------/
             |
    +--------+--------+
    | yes             | no
    |                 |
    v                 v
+------+         +--------+
| Save |         | Reject |
+------+         +--------+
```

The pieces are:

| Option | Draws or does |
| --- | --- |
| `--flow-chart` | Starts the diagram and sets its direction and placement. |
| `--flow-node ID:TEXT` | Adds a box. |
| `--flow-decision ID:TEXT` | Adds a decision shape. |
| `--flow-terminal ID:TEXT` | Adds a rounded start or end shape. |
| `--flow-edge 'FROM -> TO'` | Connects two shapes. Add `[label="yes"]` when the arrow needs a label. |

Use `direction=horizontal` to run the diagram from left to right, or
`direction=vertical` (the default) for top to bottom. `float=left|center|right`
places the diagram within the message margin; the default is `center`.
`nodesep=N` and `ranksep=N` adjust spacing. Their defaults are 0.8 and 1.0,
and both accept numbers greater than zero through 10.

IDs begin with a letter or underscore, followed by letters, digits, or
underscores. You need at least two nodes and one edge. You can write a chain
as `A -> B -> C`, and add more nodes and edges by repeating the options. The
next non-flowchart message option ends the diagram. Graphs cannot be nested.

This feature needs Graphviz's `dot` command. The tool asks Graphviz for node
order, then lays out ordinary forward arrows itself. For links that skip a
level or point backwards, it uses Graphviz's positions; those diagrams may
look different across Graphviz versions. It builds DOT internally, so the
options accept the forms shown here rather than arbitrary DOT source.

## Speech and thought bubbles

Use `--bubble` to draw a phrase with an ASCII speaker avatar. The phrase may
contain spaces; separate clauses with semicolons.

For a surprise, pass the phrase on its own:

```sh
git-compose-commit --subject 'A quick note' --bubble 'Hello there!' --stdout
```

The speaker and other choices follow your configured defaults, or are chosen
randomly when not configured. The speaker is below the bubble by default.
The entire value is treated as text unless it starts with a recognized clause
such as `phrase=` or `speaker=`. For explicit settings, use clauses:

For a speech bubble above the cat:

```sh
git-compose-commit --subject 'A quick note' \
  --bubble 'speaker=cat;style=speech;phrase=Hi;speaker-side=below;bubble-side=center;float=left' \
  --stdout
```

Output:

```text
A quick note

 .--.
( Hi )
 '--'
   \
 /\_/\
( o.o )
 > ^ <
```

The cat cannot review the patch, sadly.

For a thought bubble, the tail becomes a trail of little circles. The stick
figure looks suitably contemplative:

```sh
git-compose-commit --subject 'Consider the change' \
  --bubble 'speaker=stick-figure;style=thought;phrase=Maybe;speaker-side=below;bubble-side=center;float=left' \
  --stdout
```

Output:

```text
Consider the change

( Maybe )
    O
    o
    .
    O
   /|\
   / \
```

Choose `speaker=cat`, `speaker=dog`, or `speaker=stick-figure`. The style
may be `voice` (also `speech`) or `thought`. `speaker-side=left|right` puts the
avatar beside the bubble; `below` (the default) places it underneath. For an
avatar below, `bubble-side=left|center|right` positions the bubble relative
to it, with the speaker toward the opposite edge. `float` aligns the entire
composition within the message and accepts `left`, `center`, or `right`.
Other omitted bubble settings are chosen randomly unless configured in
`~/.compose-commit.toml`; specify settings to choose their exact values.
Phrases wrap to fit the message margin. Above the speaker, speech bubbles use
one `\` or `/` pointer aimed at the speaker's head; thought bubbles use dots.

Use `--rand-seed=N` to control all random choices made by the tool, including
bubble defaults. The same seed and arguments produce the same output when the
configuration is also unchanged; without a seed, choices vary between
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
  --list-item 'Advance the generation' --stdout
```

Output:

```text
Refresh the cache safely

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
  --footer-1 'Readers keep using the previous generation until then.' \
  --stdout
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

## How this interacts with Git

When neither `--stdout` nor `--output` is present, the tool writes the
composed message to a temporary file and runs `git commit -F` with that file.
Git handles the commit normally, including its configured cleanup and hooks.
Arguments the tool does not consume are passed to `git commit`, so you can
mix Git options with message options:

```sh
git-compose-commit --all --subject 'Refresh the cache' --signoff
```

Do not pass Git message options such as `-m`, `--message`, or `--file`; they
conflict with the generated message file. Use the compose options to build
the message instead.

`--` keeps its usual Git meaning: it ends option parsing and is passed through
to Git. Arguments after it are pathspecs:

```sh
git-compose-commit --subject 'Update the parser' -- src/parser.rs
```

## Option aliases

The examples use the unprefixed option names. For scripts that must avoid
collisions with future Git options, add `compose-` after the leading dashes:
for example, `--subject` becomes `--compose-subject` and `--table-row` becomes
`--compose-table-row`. Both forms are accepted. `--th` and `--tr` are also
available as shorter table aliases.

## Tests

Install Graphviz so its `dot` command is on `PATH`, then run the same checks
as CI:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
```

The test suite includes:

- `src/tests/mod.rs` has unit tests for parsing and formatting.
- `tests/end_to_end.rs` runs the executable in temporary Git repositories
  and checks the resulting commits.
- `tests/fixtures/` has readable argument-and-output examples. Each line
  before the blank separator is passed as one argument; the rest of the file
  is the exact expected commit message. Fixture lines are not shell-parsed.
- `tests/fixtures/errors/` records invalid argument combinations and their
  exact stderr output.

When adding a fixture, list it in `tests/end_to_end.rs`.
