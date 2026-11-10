---
id: "diff-and-baseline-2026-11-10"
status: "in-progress"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T18:29:12.000Z"
labels: ["m5", "needs-review"]
order: 3
---

# diff and baseline

`sitevet diff`, `--baseline <id|last|file>`, changes panel in both report formats.

## Requirements

- [ ] FR-HIST-04 [Must]: `sitevet diff <id-a> <id-b>` (and `--baseline <id|file>` on `audit`) shows score change per category, new failures and fixed failures. The HTML and Markdown reports gain a "changes since last run" section when a baseline is given.
  Acceptance: fixing one fixture rule between runs shows it under fixed, with the category score change.

> [!note]
> Needs a visual check by Safi. Move to `review` when done, not `done`.

Spec: `docs/spec/06-build-spec.md`, section 8.
