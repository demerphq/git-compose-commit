---
name: git-compose-tables
description: Use compact tables in commit messages when comparing structured values.
---

# Tables

Use a table when rows share the same columns and the comparison helps the
reader. Start with --table-head, then add --table-row or --table-rows. Cells
use simple delimiters, not CSV quoting. Preview the result and prefer prose
when a table adds noise. See git-compose-commit --help=tables for examples.
