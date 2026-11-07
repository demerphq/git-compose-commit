# TODO

## Release and agent adoption

- [ ] Review and merge the release preparation and agent-skill changes.
- [ ] Run the manual release workflow with publication disabled on the intended
  release commit; inspect all five real platform builds, archives, checksums,
  installers, and the generated Homebrew formula.
- [ ] Validate the PowerShell installer on Windows, the shell installer on
  macOS and ARM Linux, and cargo-binstall and Homebrew with real release assets.
- [ ] Verify crates.io name availability and publishing authorization; run an
  online publication dry run and review the crate package before publishing.
- [ ] Review the license attribution and third-party license notices.
- [ ] Decide whether to sign/notarize macOS and Windows binaries.
- [ ] When explicitly approved, tag and publish the GitHub Release and crate;
  create and publish the Homebrew tap separately if desired.
- [ ] Review the announcement drafts and community rules before posting.

See docs/releasing.md for the detailed release procedure.

# Visual and Whimsical Features for git-compose-commit

Extend git-compose-commit with additional ASCII-art and visualization
capabilities. Preserve existing CLI behavior, reuse rendering infrastructure,
and avoid unnecessary abstractions.

**General principles:**
- ASCII by default; Unicode explicitly opt-in.
- All decorative features opt-in.
- Deterministic output with fixed inputs and random seed.
- Respect --wrap-width, alignment, and existing section ordering.
- Keep output readable in Git logs, terminals, and email patches.
- Prefer extending existing functionality over introducing redundant features.

## 1. Character sets and rendering

- [x] Fix branching flowchart labels consisting only of zero-width combining
  marks; the current grid reserves a cell for them.
- [x] Add --charset=ascii|unicode, defaulting to ASCII.
- [x] Introduce shared character palettes for borders, arrows, bars, and other graphical elements.
- [x] Support Unicode box-drawing and block characters where useful.
- [x] Handle Unicode display widths correctly.
- [x] Preserve existing ASCII output exactly.
- [x] Avoid automatic terminal charset detection.

## 2. Sparklines

- [x] Add --sparkline for compact numeric sequences.
- [x] ASCII rendering using intensity characters such as .:-=+*#@.
- [x] Unicode rendering using ▁▂▃▄▅▆▇█.
- [x] Support labels, optional scaling limits, and endpoint values.
- [x] Handle negative values, constant sequences, and invalid inputs.
- [x] Support existing alignment conventions.
- [x] Reuse chart parsing/scaling where practical.
- [x] Design the renderer to return a string suitable for future use in fields and table cells.

Example:

~~~text
Latency : @#*+=:.
Memory  : .:-=+*#
Errors  : ##*=::..
~~~

## 3. Additional speech-bubble characters

Extend the existing bubble system with:

- [x] Cow
- [x] Robot
- [x] Owl
- [x] Dragon
- [x] Penguin
- [x] Rubber duck
- [x] Ghost

Reuse existing bubble layout, random selection, and configuration mechanisms.
Keep artwork compact and recognizable.

## 4. Banners and headings

- [x] Add decorative section headings or banners.
- [x] Support a few styles: plain, box, double, heavy, retro.
- [x] Support alignment and configured width.
- [x] Use shared character palettes for Unicode alternatives.
- [x] Avoid excessively large ASCII-art typography.

Example:

~~~text
====================================
           BREAKING CHANGE
====================================
~~~

## 5. Commit stamps

Add compact, optionally labeled ASCII-art stamps:

- [x] Approved / OK
- [x] Warning / danger
- [x] Tests passed
- [x] Achievement unlocked
- [x] Bug eliminated
- [x] RIP / tombstone
- [x] Deprecated
- [x] Performance improvement

Reuse existing alignment and text-rendering capabilities.

## 6. Progress bars and checklists

- [x] Add percentage- and fraction-based progress bars.
- [x] Support configurable widths and labels.
- [x] Add completed/incomplete checklist items.
- [x] Reuse list-rendering infrastructure where appropriate.
- [x] Design progress-bar rendering for possible inline use.

Example:

~~~text
Migration: [########............] 40%

[x] Schema migration
[x] Backfill
[ ] Remove dual writes
[ ] Legacy cleanup
~~~

## 7. Badges

- [x] Add compact ASCII badges with custom labels, including status labels.
- [ ] **Paused for review:** predefined badge names duplicate custom labels and
  the named stamps; decide whether aliases add enough value to justify them.
- [x] Support multiple badges on one line.
- [x] Provide plain and boxed styles.
- [x] Integrate with Unicode mode where appropriate.
- [x] Avoid interfering with conventional commit subjects.

Example:

~~~text
[ PERF ] [ TESTED ] [ READY ]
~~~

## 8. Fortune cookies

- [x] Add a small collection of original programming jokes and aphorisms.
- [x] Support explicit selection and seeded random selection.
- [x] Reuse existing boxes or speech bubbles.
- [x] Keep entirely opt-in.

## 9. Visual themes

Explore optional themes such as plain, retro, terminal, comic, and fancy.

**Paused for review:** themes would overlap with --charset and the banner and
badge styles. Decide whether a coherent combination provides enough value to
justify another setting.

- [ ] Share styling primitives across features.
- [ ] Keep existing appearance as the default.
- [ ] Avoid altering message semantics or numerical interpretation.
- [ ] Implement only if themes provide meaningful benefits beyond --charset.

## 10. Reusable inline rendering

Investigate a lightweight internal abstraction allowing generated fragments
to appear inside existing fields and table cells.

**Investigated:** cli-compose-text can generate a fragment for ordinary shell
command substitution in a field or table cell. This works with sparklines,
progress bars, and badges without adding a second interpolation language; see
the reference example. **Paused for review:** a separate structured
composition API overlaps with that existing mechanism; consider it later if
shell composition proves insufficient.

Candidates:

- Sparklines
- Progress bars
- Badges
- Compact status indicators

For example:

~~~text
+----------+---------+----------+
| Metric   | Current | Trend    |
+----------+---------+----------+
| Latency  | 42 ms   | @#*+=:.  |
| Memory   | 128 MB  | .:-=+*#  |
+----------+---------+----------+
~~~

Avoid a general-purpose template language or incompatible feature-specific
interpolation syntaxes. Preserve existing CLI compatibility.

## 11. Documentation and tests

- [x] Add relevant examples to --help=TOPIC.
- [x] Update corresponding agent skills.
- [x] Test ASCII and Unicode output, alignment, wrapping, and edge cases.
- [x] Preserve existing output through regression tests.
- [x] Keep the README concise; detailed syntax belongs in topic help or reference documentation.

## Suggested priority

1. Character palettes and Unicode support.
2. Sparklines.
3. Additional talking characters.
4. Banners, stamps, and badges.
5. Progress bars and checklists.
6. Inline rendering integration.
7. Themes and fortune cookies.

Treat this as a feature backlog rather than a mandate to implement everything
simultaneously. Choose straightforward implementations, reuse existing
components, and avoid expanding the CLI where existing options already solve
the problem.
