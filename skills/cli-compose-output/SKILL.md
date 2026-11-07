---
name: cli-compose-output
description: Write composed text to stdout or a file, or prepare it for Git.
---

# Output

Use cli-compose-text for text on stdout; --output FILE writes a file instead
and overwrites it. Neither invokes Git. The same message options also work
with git-compose-commit, whose --stdout and --output options skip the commit.
See cli-compose-text --help=output for details.

For an authorized Git commit, use the git-compose-commit skill and follow the
user's instructions and repository conventions.
