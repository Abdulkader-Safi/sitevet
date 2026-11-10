---
id: "render-helpers-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m3"]
order: 4
---

# Render helpers

`render`, `render_with`, `audit_to_html`, `audit_to_markdown`, `audit_score`.

## Requirements

- [ ] FR-API-05 [Must]: Convenience functions must render a report or return just the score: `render(&report, format)`, plus one-call helpers equivalent to the TS package's `auditToHtml`, `auditToMarkdown` and `auditScore`.
  Acceptance: each helper's output equals `render` on the report from `audit` for the same site.

Spec: `docs/spec/06-build-spec.md`, section 8.
