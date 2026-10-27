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

## Compose a message

The subject is always the first line, wherever `--subject` appears in the
arguments. Other message sections appear in argument order, separated by
blank lines.

| Option | Output |
| --- | --- |
| `--subject TEXT` | Subject line. May be specified once. |
| `--text TEXT` | Prose paragraph. Whitespace is normalized and text wraps at 72 characters. |
| `--detail TEXT` | Unwrapped text block with four spaces added before each line. |
| `--line TEXT` | One unwrapped line. Embedded newlines are rejected. |

Repeat `--text`, `--detail`, and `--line` to add sections. Values may follow
the option or use `=`:

```sh
--text 'A paragraph of prose.'
--text='A paragraph of prose.'
```

Words longer than the wrap margin are kept intact. Git may apply its normal
commit-message cleanup after the wrapper supplies the message.

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
| `--th`, `--table-head` | `--compose-th`, `--compose-table-head` |
| `--tr`, `--table-row` | `--compose-tr`, `--compose-table-row` |
| `--table-rows` | `--compose-table-rows` |
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
