---
id: "layout-shifts-and-slow-resources-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-16T10:52:42.000Z"
labels: ["m10"]
order: 2
---

# Layout shifts and slow resources

## Requirements

- [x] FR-PERF-03 [Must]: Layout shift event count and largest single shift. Slow resource list: every request over a threshold (default 300 ms), or a clean "none exceeded" result.
  Acceptance: a fixture with a delayed image lists it.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- Layout shift count and the largest shift. Requests over `slow_resource_ms` listed; the 600 ms fixture image is in the list, and a clean run says "None exceeded the threshold".
