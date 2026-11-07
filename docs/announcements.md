# Announcement drafts

Drafts only. Verify availability, links, and each community's posting rules
before using them. No announcement has been posted.

## Show HN

**Title:** Show HN: git-compose-commit – structured commit messages from a CLI

I made a CLI that builds Git commit messages from structured arguments. It
wraps paragraphs and can format lists, tables, footers, and diagrams before
passing the message to `git commit`. A preview option prints the result
without committing. It makes no AI calls; it is also intended to be usable
by coding agents, with embedded skills and focused help topics. It can draw
a talking cat, which remains unavailable for code review.

Repository: https://github.com/demerphq/git-compose-commit

## r/commandline

**Title:** A CLI for composing structured Git commit messages

I built `git-compose-commit` for messages that have outgrown a single `-m`:
pass a subject, paragraphs, lists or tables as arguments, preview the result,
then pass it to Git. It also does charts and the occasional talking animal.
The program formats text locally and does not generate content or call an AI
service. Feedback on the command-line interface is welcome.

https://github.com/demerphq/git-compose-commit

## r/rust

**Title:** git-compose-commit: a Rust CLI for structured commit messages

I wrote a small Rust CLI that composes and wraps commit messages from command
line options, with Git passthrough and a preview mode. It includes an embedded
Agent Skills interface for coding tools and an optional Graphviz integration
for branching flowcharts. I'd appreciate feedback on portability, Unicode
display width handling, and the CLI design.

https://github.com/demerphq/git-compose-commit

## Coding-agent communities

**Title:** A local commit-message formatter with discoverable agent skills

`git-compose-commit` lets an agent compose message content using structured
arguments, preview the formatted result, and use Git when the user has
authorized a commit. It does not call an AI service. The executable can list
and export its own Agent Skills, including focused instructions for tables,
lists, output, and other features. It supports Codex and Claude Code skill
directories; the repository has installation examples.

https://github.com/demerphq/git-compose-commit
