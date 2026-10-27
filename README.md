# git-compose-commit

`git-compose-commit` assembles a commit message from named sections, then runs
`git commit` with that message. Any other arguments are passed through to Git.

```sh
git-compose-commit --compose-subject 'Fix cache invalidation' \
  --compose-text 'Refresh the cache after a successful write so later readers see the new value.' \
  --compose-detail 'The cache uses generation numbers to detect stale entries.' \
  --compose-table-head 'Name,State,Count' \
  --compose-table-row 'cache,updated,3' \
  --compose-list=numbered \
  --compose-list-item 'Refresh the cache' \
  --compose-list-item 'Verify the new value' \
  --compose-list-end \
  --compose-list=bullets \
  --compose-list-item 'Keep the update atomic' \
  --compose-text 'Refresh the cache after writes [#1]' \
  --compose-footer-1 'The generation counter now advances only after a successful write.' \
  --compose-line 'Fixes: #123' \
  --all
```

The subject is always the first line. `--compose-text` paragraphs are whitespace
normalized and wrapped at 72 columns. `--compose-detail` content is preserved as
literal text and indented by four spaces. `--compose-line` is emitted as one
unwrapped line and rejects embedded newlines. Sections are separated by blank
lines. Each section option can be repeated; `--compose-subject` must occur at most
once. Values can be supplied as `--compose-subject=value` or as the next argument.
Tables use comma-separated cells, such as `--compose-table-head=Name,State` and
`--compose-table-row=cache,updated`. Each row must have the same number of cells as
its header. A new table header starts a new table section, and rows are added
to the most recent header. Tables have ASCII borders and columns sized to fit
their contents. The short forms `--compose-th` and `--compose-tr` are also available.
For several rows at once, `--compose-table-rows` separates rows with semicolons,
for example `--compose-table-rows='cache,ready;worker,waiting'`.

Text and line sections can refer to numbered footers with `[#N]`, for example
`--compose-text 'See the retry behavior [#1]' --compose-footer-1 'Retries use bounded
exponential backoff.'`. References inside backtick-delimited code spans are
ignored. Every reference must have a matching `--compose-footer-N`, and every
footer must be referenced. Footers are appended after the other sections in
number order, with their labels aligned and their text wrapped to fit.

Lists use `--compose-list=numbered` or `--compose-list=bullets`, followed by one or more
`--compose-list-item` options. A list item without an active list starts a
numbered list automatically. Starting a list inside an open list nests it; use
`--compose-list-end` to return to the parent. Lists also end automatically when
another message section begins or the arguments end.

Use `--` to end compose-option parsing; everything after it, including
pathspecs that look like compose options, is passed directly to Git.

Unprefixed aliases are also supported: `--subject`, `--text`, `--detail`,
`--line`, `--table-head`, `--table-row`, `--table-rows`, `--th`, `--tr`,
`--list`, `--list-item`, `--list-end`, and `--footer-N`. They may conflict
with options added by a future Git version, so prefer the `--compose-*` forms
for scripts and long-term use.

## Tests

Run `cargo test`. Unit tests for the message formatting logic are in
`src/tests/mod.rs`; end-to-end tests that invoke the executable and inspect
commits in temporary Git repositories are in `tests/end_to_end.rs`.
