---
name: git-compose-output
description: Preview composed messages and pass authorized commits to Git.
---

# Preview and commit

Use compose-text to write a message to stdout without invoking Git, or use
--output FILE to write it to a file. The wrapper's --stdout and --output also
skip Git. File output overwrites FILE. These modes do not accept Git options.
When authorized, rerun without the output option and include the intended
Git options or pathspec. The tool invokes git commit with a temporary message
file; Git cleanup and hooks apply. Do not pass Git's -m or -F.

Run git-compose-commit --help=output for details. Follow repository and user
instructions and do not create commits without authorization.
