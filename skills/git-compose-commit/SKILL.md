---
name: git-compose-commit
description: Compose structured Git commit messages with git-compose-commit when a commit is authorized.
---

# Compose a commit message

Follow the user's instructions and the repository's AGENTS.md or CLAUDE.md.
Create a commit only when authorized. Inspect the staged changes and local
commit conventions first. Choose an imperative, concrete subject and explain
the change and its reason where useful.

Preview with compose-text --subject 'Summarize the change' --text 'Explain why
it matters.' It writes to stdout without invoking Git; --output writes a file.
Check the result, then use git-compose-commit with the same message options and
the intended Git options when a commit is authorized. Git hooks and cleanup can
still affect the actual commit. The wrapper's --stdout also previews.

Use --help=message, --help=output, or another topic for exact syntax and
examples. Other skills cover tables, lists, footers, charts, flowcharts, and
bubbles. Do not add decorative sections unless they help the message.
