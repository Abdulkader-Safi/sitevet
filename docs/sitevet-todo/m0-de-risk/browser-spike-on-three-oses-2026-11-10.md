---
id: "browser-spike-on-three-oses-2026-11-10"
status: "in-progress"
priority: "critical"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:23:45.000Z"
labels: ["m0"]
order: 1
---

# Browser spike on three OSes

Launch the installed Chrome with chromiumoxide 0.9. Load a page, inject axe-core 4.14 and read violations, read LCP and CLS through `PerformanceObserver`, take a full-page and a viewport screenshot. Cancel mid-load and confirm no Chrome process remains after 1 s. Run it on macOS, Linux and Windows. If chromiumoxide fails, try chromey (same API).

Spec: `docs/spec/06-build-spec.md`, section 8.
