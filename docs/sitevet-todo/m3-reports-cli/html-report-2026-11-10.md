---
id: "html-report-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m3", "needs-review"]
order: 3
---

# HTML report

Self-contained file, neutral styling with light and dark themes, `<details>` sections, `dir="auto"` on page text, WCAG 2.2 AA, readable at 360 px. No company branding.

## Requirements

- [ ] FR-REPORT-03 [Must]: HTML output: one self-contained file with inline CSS and no external requests, an overall score badge, per-category sections, colour-coded findings with text labels, metric tables with "showing N of M", and the site-wide sections. Neutral sitevet styling, no company branding.
  Acceptance: the file opens offline with zero network requests and passes an axe check at WCAG 2.2 AA.
- [ ] FR-REPORT-04 [Must]: Text quoted from a right-to-left page must display in its own direction without breaking the layout.
  Acceptance: an Arabic fixture's title shows right to left in the HTML report.

> [!note]
> Needs a visual check by Safi. Move to `review` when done, not `done`.

Spec: `docs/spec/06-build-spec.md`, section 8.
