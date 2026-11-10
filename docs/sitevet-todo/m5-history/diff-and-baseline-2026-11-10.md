---
id: "diff-and-baseline-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T21:00:00.000Z"
labels: ["m5", "needs-review"]
order: 3
---

# diff and baseline

`sitevet diff`, `--baseline <id|last|file>`, changes panel in both report formats.

## Requirements

- [x] FR-HIST-04 [Must]: `sitevet diff <id-a> <id-b>` (and `--baseline <id|file>` on `audit`) shows score change per category, new failures and fixed failures. The HTML and Markdown reports gain a "changes since last run" section when a baseline is given.
  Acceptance: fixing one fixture rule between runs shows it under fixed, with the category score change.

> [!note]
> Needs a visual check by Safi. Move to `review` when done, not `done`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`sitevet::diff(&baseline, &current)` in the library (no feature needed) returns a `ReportDiff`: overall and per-category score change, new failures and fixed findings matched by their stable `key` (fail and warn count as failing), and pages added and removed. `sitevet diff <a> <b> [--format text|md|json]`, and `audit --baseline <id|last|file.json>` (also `baseline` under `[cli]`) fills `report.changes`. Both reports then open with a "Changes since run N" panel: score change, the categories that moved, fixed and new issues.

Test: a fixture page loses its missing title between two runs; `diff 1 2` lists `core-seo/title-present` under fixed with core-seo's score going up, and `--baseline last` compares with the newest run. Live: two runs of abdulkadersafi.com (5 pages, then 8) gave "10 new issues, 3 pages added".

Needs Safi's look at the changes panel.

Reviewed: Safi checked the panel on their own runs (reports 3 to 5, 2026-11-10). That review found checks that ran in only one run counted as fixed or new; 0.2.1 leaves them out and shows a "not compared" count.
