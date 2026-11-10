---
id: "html-report-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T18:26:05.000Z"
labels: ["m3", "needs-review"]
order: 3
---

# HTML report

Self-contained file, neutral styling with light and dark themes, `<details>` sections, `dir="auto"` on page text, WCAG 2.2 AA, readable at 360 px. No company branding.

## Requirements

- [x] FR-REPORT-03 [Must]: HTML output: one self-contained file with inline CSS and no external requests, an overall score badge, per-category sections, colour-coded findings with text labels, metric tables with "showing N of M", and the site-wide sections. Neutral sitevet styling, no company branding.
  Acceptance: the file opens offline with zero network requests and passes an axe check at WCAG 2.2 AA.
- [x] FR-REPORT-04 [Must]: Text quoted from a right-to-left page must display in its own direction without breaking the layout.
  Acceptance: an Arabic fixture's title shows right to left in the HTML report.

> [!note]
> Needs a visual check by Safi. Move to `review` when done, not `done`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/templates/report.html` with `assets/report.css` inlined. Header with the URL, date and a score badge (number and word), a category grid, one `<details>` per page (first open) with findings grouped by category, status words beside the colours, evidence in `<pre dir="auto">`, fix text, and a collapsed "Measurements" section with tables. Blocked URLs and a version footer. Light and dark themes through `prefers-color-scheme`, system fonts, no script, no images, no external requests (a test checks for `src=`, `<link`, `<script`, `url(`, `@import`).

Checked on a live 15-page audit of abdulkadersafi.com: Lighthouse accessibility (axe-core) 100 in dark mode on desktop and 100 in light mode at 360 px. The narrow layout got a smaller badge and a two-column category grid after that check.

FR-REPORT-04: page text sits in `dir="auto"` elements; the Arabic fixture's title shows as right-to-left evidence. Escaping is askama's; a test feeds `<script>` and pipes through a page.

Needs Safi's look. Render one with `sitevet audit <url> --out report.html`.

## Review

Safi opened the report from the released CLI on 2026-11-10 (100 pages of abdulkadersafi.com, score 92, 2.4 MB with `--browser`). It rendered as built; no layout issues raised. Found in use: LinkedIn answers bots with status 999, which `links/broken-external` counts as broken on every page. A 0.1.1 fix is proposed (report it as unverifiable); logged in PROGRESS.md.
