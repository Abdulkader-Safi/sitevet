---
id: "static-performance-checks-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m10"]
order: 3
---

# Static performance checks

## Requirements

- [ ] FR-PERF-04 [Must]: Static performance checks that need no browser: font loading (`font-display`, preloads, font file count), HTTP/2 or HTTP/3 in use, resource hints (preconnect, dns-prefetch) for third-party origins, third-party script count and weight.
  Acceptance: each check passes and fails on matching fixtures.

Spec: `docs/spec/06-build-spec.md`, section 8.
