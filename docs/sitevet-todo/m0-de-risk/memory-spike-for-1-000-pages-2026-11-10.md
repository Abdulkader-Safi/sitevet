---
id: "memory-spike-for-1-000-pages-2026-11-10"
status: "in-progress"
priority: "critical"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:23:45.000Z"
labels: ["m0"]
order: 4
---

# Memory spike for 1,000 pages

Parse 1,000 generated fixture pages through `spawn_blocking`, extract a PageData-like struct, drop the DOM, measure peak resident memory. Target: projected under 500 MB (NFR-02).

Spec: `docs/spec/06-build-spec.md`, section 8.
