---
id: "orphans-link-graph-and-click-depth-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m8"]
order: 3
---

# Orphans, link graph and click depth

## Requirements

- [ ] FR-SITE-03 [Must]: Orphan pages: in the sitemap but not linked from any crawled page.
  Acceptance: the sitemap-only fixture page is reported as orphaned.
- [ ] FR-SITE-04 [Must]: Internal link graph with inbound link count per URL, and click depth per page with a warning past a threshold (default 3).
  Acceptance: fixture counts match the known graph.

Spec: `docs/spec/06-build-spec.md`, section 8.
