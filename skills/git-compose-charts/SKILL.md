---
name: git-compose-charts
description: Add horizontal or vertical bar charts to commit messages.
---

# Bar charts

Use charts only when the values communicate something useful in the commit.
Choose --hbar-chart or --vbar-chart and provide at least two label:value
items. Set float and size explicitly when layout needs to be repeatable.
For a compact numeric series use --sparkline; --charset=unicode opts into
Unicode blocks. Negative and constant sparkline values are supported.
Use --progress for a percentage or fraction when a single progress bar helps.
Preview the result; ASCII charts can be wider than a typical commit message.
See git-compose-commit --help=charts for syntax and output.
