# git-compose-commit

`git-compose-commit` assembles a commit message from named sections, then runs
`git commit`. Git commit options are passed through.

```sh
git-compose-commit --subject 'Fix cache invalidation' \
  --text 'Refresh the cache after a successful write so later readers see the new value.' \
  --detail 'The cache uses generation numbers to detect stale entries.' \
  --th 'Name,State,Count' \
  --tr 'cache,updated,3' \
  --numlist \
  --list-item 'Refresh the cache' \
  --list-item 'Verify the new value' \
  --bullets \
  --list-item 'Keep the update atomic' \
  --text 'The generation counter now advances after a successful write [#1]' \
  --footer-1 'This prevents readers from observing a partially updated value.' \
  --line 'Fixes: #123' \
  --all
```

## Options

The short, unprefixed options are convenient for interactive use. Their
namespaced `--compose-*` equivalents are available for scripts and to avoid
possible conflicts with future `git commit` options.

| Short option | Namespaced option |
| --- | --- |
| `--subject` | `--compose-subject` |
| `--text` | `--compose-text` |
| `--detail` | `--compose-detail` |
| `--line` | `--compose-line` |
| `--th` | `--compose-th` or `--compose-table-head` |
| `--tr` | `--compose-tr` or `--compose-table-row` |
| `--table-rows` | `--compose-table-rows` |
| `--footer-N` | `--compose-footer-N` |
| `--list` | `--compose-list` |
| `--list-item` | `--compose-list-item` |
| `--list-end` | `--compose-list-end` |
| `--numlist` | `--compose-numlist` |
| `--bullets` | `--compose-bullets` |

The longer unprefixed table aliases `--table-head` and `--table-row` are also
accepted. Table shorthand works both ways: `--th`/`--tr` and
`--compose-th`/`--compose-tr` are supported, as are
`--compose-table-head`/`--compose-table-row`.

Values can be written as `--text='paragraph text'` or as the next argument.
The subject is always the first line. Text paragraphs are whitespace
normalized and wrapped at 72 columns. Detail content is emitted literally
with a four-space prefix on each line. Line content stays unwrapped and must
not contain a newline. Message sections are separated by blank lines.

Tables use commas between cells and semicolons between rows:

```sh
git-compose-commit --th 'Name,State' --table-rows='cache,ready;worker,waiting'
```

Cells are padded to fit and the table is drawn with ASCII borders. Each row
must have the same number of cells as the header. Repeated `--tr` options add
rows individually; `--table-rows` adds several at once.

Lists default to numbered when started by `--list-item`. Use `--numlist` or
`--list=numbered` for numbered lists and `--bullets` or `--list=bullets` for
bullet lists. Starting another list inside one nests it; use `--list-end` to
return to its parent. Lists close automatically when another message section
begins or the arguments end.

Text and line sections can reference numbered footers with `[#N]`. Each
reference needs a matching `--footer-N`, and every footer must be referenced.
References inside backtick-delimited code spans are ignored. Footers appear at
the end in number order, with aligned labels and wrapped text.

Use `--` to stop wrapper option parsing. The `--` and everything after it are
passed directly to Git; Git treats arguments after its separator as pathspecs.

## Tests

Run `cargo test`. Unit tests for formatting logic are in `src/tests/mod.rs`.
End-to-end tests in `tests/end_to_end.rs` invoke the executable in temporary
Git repositories. Golden fixtures in `tests/fixtures/` contain one argument
per line, a blank separator, and the exact expected commit message.
