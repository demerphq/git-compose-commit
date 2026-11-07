# git-compose-commit

A CLI for composing well-formatted Git commit messages, designed for humans
and AI coding agents.

`git commit -m` is handy until a message needs several paragraphs, lists, or
other structure. `cli-compose-text` builds formatted text from command-line
arguments; `git-compose-commit` uses the same composer and passes the result
to Git. There is no AI service
involved: an agent writes the content; this tool formats it. Bubble choices
can be random; set them explicitly or use a seed for repeatable output.

## Example

Preview a message before committing:

```sh
cli-compose-text --subject 'Refresh cache after writes' \
  --text 'Keep readers on the previous generation until the new value passes validation.' \
  --bullets --list-item 'Validate the new value' \
  --list-item 'Publish the new generation' \
  --line 'Fixes: #123'
```

Output:

```text
Refresh cache after writes

Keep readers on the previous generation until the new value passes
validation.

* Validate the new value
* Publish the new generation

Fixes: #123
```

`cli-compose-text` writes to stdout by default; use `--output FILE` to write a
file. It never invokes Git. To create an authorized commit, use the same
message options with `git-compose-commit`, adding any Git options you need.
The wrapper also retains its `--stdout` and `--output` preview modes.

## Installation

On macOS or Linux with Homebrew:

```sh
brew install demerphq/tap/git-compose-commit
```

With a Rust toolchain:

```sh
cargo install git-compose-commit --locked
```

You can also install from a checkout:

```sh
cargo install --path .
```

For Windows and other binary installation options, see the
[releases](https://github.com/demerphq/git-compose-commit/releases) and
[installation instructions](docs/releasing.md#installation-after-publication).
Git is required for `git-compose-commit`. Graphviz's `dot` is optional and
only needed for branching flowcharts.

## What it can do

- Compose wrapped paragraphs, literal details, fields, and callouts.
- Arrange lists, tables, and numbered footers.
- Draw bar charts, sparklines, progress bars, and linear or branching flowcharts.
- Add checklists, banners, badges, and stamps when they help the message.
- Add a speech or thought bubble, complete with a talking animal. The cat
  cannot review the patch, sadly.
- Forward Git options and pathspecs to `git commit`.

## Using with coding agents

Skills help an agent choose the right options without loading the full
reference into its context:

```sh
git-compose-commit --agent-usage-list-skills
git-compose-commit --agent-usage-show-skill=git-compose-commit
git-compose-commit --help=tables
```

See [agent integration](docs/agents.md) for Codex and Claude Code skill
installation, example project instructions, and the export format. Skills
respect project conventions and do not authorize commits by themselves.

## Documentation

- [Complete usage reference and examples](docs/reference.md)
- [Agent integration and skill export convention](docs/agents.md)
- [Release preparation](docs/releasing.md)
- `cli-compose-text --help` for the text-only interface.
- `git-compose-commit -h` for a quick synopsis; `--help` or `--man` for the
  full guide; `--help=TOPIC` for a focused guide such as `--help=flowcharts`.

Configuration is optional: `~/.compose-commit.toml` can set wrapping and
bubble defaults. See the reference for the available settings.

## Development

Install Graphviz for branching flowchart tests, then run:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
```
