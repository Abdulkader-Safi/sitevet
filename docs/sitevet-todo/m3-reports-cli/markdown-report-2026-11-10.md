---
id: "markdown-report-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:16:46.000Z"
labels: ["m3"]
order: 2
---

# Markdown report

askama template, escaped page text, insta snapshot.

## Requirements

- [x] FR-REPORT-02 [Must]: Markdown output: score summary and per-category headings, each with its findings and metric tables.
  Acceptance: a snapshot test of the fixture report passes, and the output renders on GitHub.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

askama template `crates/sitevet/templates/report.md`: summary line, category table, per-page sections with issues grouped by category (passes counted, not listed), measurement tables with "showing N of M", blocked URLs, and a footer with the versions. Page text is escaped in Rust before it reaches the template: pipes, backticks, asterisks, underscores, angle and square brackets, `#` and `~`, and newlines become spaces. insta snapshot `tests/snapshots/report__basic_markdown.snap`, with the port, time and timings replaced.
