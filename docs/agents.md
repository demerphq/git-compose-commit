# Agent integration

Both executables include the same eleven [Agent Skills](https://agentskills.io/specification):
one cover skill for general text, one for Git commits, and focused skills for
message composition, output, tables, lists, footers, charts, flowcharts,
visual decorations, and bubbles. They tell agents when and how
to use the tool; `--help=TOPIC` supplies detailed syntax and examples.

List and inspect skills without invoking Git or loading user configuration:

```sh
git-compose-commit --agent-usage-list-skills
git-compose-commit --agent-usage-show-skill=git-compose-commit
```

## Install skill documents

With a POSIX shell, choose the appropriate destination from the table below.
This installs the skill documents directly from the executable; no network
download is needed. Review existing files first: this command overwrites them.

```sh
skill_dir=.agents/skills
git-compose-commit --agent-usage-list-skills | while IFS= read -r skill; do
  mkdir -p "$skill_dir/$skill"
  git-compose-commit "--agent-usage-show-skill=$skill" > "$skill_dir/$skill/SKILL.md"
done
```

| Agent | Project | User |
| --- | --- | --- |
| Codex | `.agents/skills` | `~/.agents/skills` |
| Claude Code | `.claude/skills` | `~/.claude/skills` |

For PowerShell, choose the destination and run:

```powershell
$skillDir = '.claude/skills'
git-compose-commit --agent-usage-list-skills | ForEach-Object {
  $directory = Join-Path $skillDir $_
  New-Item -ItemType Directory -Force $directory | Out-Null
  $text = git-compose-commit "--agent-usage-show-skill=$_"
  [System.IO.File]::WriteAllText((Join-Path (Resolve-Path $directory) 'SKILL.md'),
    (($text -join "`n") + "`n"), [System.Text.UTF8Encoding]::new($false))
}
```

Restart or reload your agent if it does not discover newly installed skills.
To install every supporting file as well, from a checkout use:

```sh
git-compose-commit --agent-usage-show-skill=ALL | \
  python3 scripts/extract-agent-skills.py .agents/skills
```

The extractor refuses to overwrite existing files. Set the destination to the
agent and scope you want. The complete export includes the reference extractor
as a supporting file in the general skill.

## Suggested project instructions

For `AGENTS.md` or `CLAUDE.md`, adapt this to the project's own rules:

```text
When a commit is authorized, use git-compose-commit to compose the message.
Follow the user's instructions and this repository's commit conventions.
Preview with --stdout when useful. Use --help=TOPIC for detailed syntax.
Do not create a commit solely because this instruction exists.
```

## A convention for other CLI tools

These options are a proposed convention, not an established standard:

- `--agent-usage-list-skills` writes UTF-8 skill names, one per line.
- `--agent-usage-show-skill=NAME` writes that skill's complete `SKILL.md`.
- `--agent-usage-show-skill=ALL` exports all files; uppercase `ALL` is reserved.

These operations are standalone, deterministic, and have no side effects.
Errors go to stderr with a nonzero exit status. They are mutually exclusive
with other options, so output is always machine-readable.

### ALL stream, version 1

The byte stream starts with `AGENT-USAGE/1\n`. Each file then has an ASCII
header `PATH_BYTES CONTENT_BYTES\n`, followed immediately by exactly
`PATH_BYTES` UTF-8 path bytes and `CONTENT_BYTES` content bytes. Paths are
relative to the skills directory, use `/`, and include the skill directory.
There is no separator after either byte sequence. A `0 0\n` header ends the
stream; no bytes follow it. Sizes are decimal byte counts, not character
counts. Records have a deterministic order. A zero path length is reserved
for the terminator. Consumers must reject absolute paths, traversal, duplicate
paths, truncated input, and unwanted overwrites.

For interoperability with the reference extractor, use path components made
of ASCII letters, digits, underscores, dots, or hyphens; `.` and `..` are not
allowed as components.

[extract-agent-skills.py](../scripts/extract-agent-skills.py) is a simple
reference implementation. Other tools can use this framing and list their own
skill directories and supporting files.
