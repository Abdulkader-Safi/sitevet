---
id: "browser-spike-on-three-oses-2026-11-10"
status: "done"
priority: "critical"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T09:42:25.000Z"
labels: ["m0"]
order: 1
---

# Browser spike on three OSes

Launch the installed Chrome with chromiumoxide 0.9. Load a page, inject axe-core 4.14 and read violations, read LCP and CLS through `PerformanceObserver`, take a full-page and a viewport screenshot. Cancel mid-load and confirm no Chrome process remains after 1 s. Run it on macOS, Linux and Windows. If chromiumoxide fails, try chromey (same API).

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

Passes on macOS (local and CI), Linux and Windows with chromiumoxide 0.9.1. axe-core 4.14.0 finds color-contrast, image-alt and label. LCP and CLS read through PerformanceObserver. Viewport and full-page screenshots work. Hard-kill cancel leaves 0 Chrome processes in 33 to 515 ms. chromey not needed. Two findings for M4: open tabs as about:blank and bring_to_front before goto (or CLS reads 0), and wait for observer entries instead of sleeping.

Numbers in `PROGRESS.md`, section M0.
