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
download is needed. It refuses to overwrite existing skill documents or write
through a skill-directory symlink. Review and remove an old installation
yourself before replacing it.

```sh
(
  set -eu
  skill_dir=.agents/skills
  skills=$(git-compose-commit --agent-usage-list-skills)
  for skill in $skills; do
    if [ -L "$skill_dir/$skill" ] || [ -e "$skill_dir/$skill/SKILL.md" ] ||
       [ -L "$skill_dir/$skill/SKILL.md" ]; then
      echo "Already exists or is a symlink: $skill_dir/$skill" >&2
      exit 1
    fi
  done
  for skill in $skills; do
    mkdir -p "$skill_dir/$skill"
    (set -C; git-compose-commit "--agent-usage-show-skill=$skill" > "$skill_dir/$skill/SKILL.md")
  done
)
```

| Agent | Project | User |
| --- | --- | --- |
| Codex | `.agents/skills` | `~/.agents/skills` |
| Claude Code | `.claude/skills` | `~/.claude/skills` |

For PowerShell, choose the destination and run:

```powershell
$skillDir = '.claude/skills'
$skills = @(git-compose-commit --agent-usage-list-skills)
if ($LASTEXITCODE -ne 0) { throw 'Could not list skills' }
foreach ($skill in $skills) {
  $directory = Join-Path $skillDir $skill
  $item = Get-Item -LiteralPath $directory -Force -ErrorAction SilentlyContinue
  if (($item -and ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) -or
      (Test-Path -LiteralPath (Join-Path $directory 'SKILL.md'))) {
    throw "Already exists or is a symlink: $directory"
  }
}
foreach ($skill in $skills) {
  $directory = Join-Path $skillDir $skill
  New-Item -ItemType Directory -Force $directory | Out-Null
  $text = git-compose-commit "--agent-usage-show-skill=$skill"
  if ($LASTEXITCODE -ne 0) { throw "Could not export $skill" }
  $path = Join-Path (Resolve-Path $directory) 'SKILL.md'
  $file = [System.IO.File]::Open($path, [System.IO.FileMode]::CreateNew)
  $writer = [System.IO.StreamWriter]::new($file, [System.Text.UTF8Encoding]::new($false))
  try { $writer.Write(($text -join "`n") + "`n") } finally { $writer.Dispose() }
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
as a supporting file in the Git cover skill.

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
