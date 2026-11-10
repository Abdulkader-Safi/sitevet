---
id: "cli-progress-thresholds-and-exit-codes-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m3", "needs-review"]
order: 6
---

# CLI progress, thresholds and exit codes

Per-page lines, summary block, `--fail-under`, `--fail-on`, exit 0, 1 or 2, `NO_COLOR`, `--quiet`, `--plain`. Snapshot tests.

## Requirements

- [ ] FR-CLI-03 [Must]: Default progress shows one line per page (`3/100  https://site/about  92/100`) and a summary with the overall score and the counts of failures per category.
  Acceptance: snapshot test of stderr on a fixture site.
- [ ] FR-CLI-04 [Must]: `--fail-under <score>` and `--fail-on error|warning` for CI. Exit codes: 0 when the audit ran and passed thresholds, 1 when a threshold failed, 2 when the audit could not run (bad URL, unreachable start page, bad config, missing browser).
  Acceptance: each exit code is produced by a matching test case.
- [ ] FR-CLI-05 [Must]: Output respects `NO_COLOR`, prints status words (PASS, FAIL, WARN) next to colours, and has `--quiet` and `--plain` modes (no animation, no colour).
  Acceptance: with `NO_COLOR=1`, stderr contains no ANSI escape codes.

> [!note]
> Needs a visual check by Safi. Move to `review` when done, not `done`.

Spec: `docs/spec/06-build-spec.md`, section 8.
