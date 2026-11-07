---
name: git-compose-flowcharts
description: Illustrate linear or branching processes in commit messages.
---

# Flowcharts

Use a flowchart when a process is clearer as a diagram. Linear chains use
--hflow-chart or --vflow-chart. For branching graphs use --flow-chart and
define nodes and edges. Branching flowcharts require Graphviz's dot command.
Preview and check the diagram's width and alignment. See
git-compose-commit --help=flowcharts for syntax and examples.
