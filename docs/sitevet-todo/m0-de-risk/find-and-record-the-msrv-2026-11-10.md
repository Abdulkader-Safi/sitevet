---
id: "find-and-record-the-msrv-2026-11-10"
status: "done"
priority: "critical"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T09:42:25.000Z"
labels: ["m0"]
order: 5
---

# Find and record the MSRV

Run `cargo msrv find` on the spike's dependency set. It must be 1.90 or older. Record it in `PROGRESS.md`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

cargo msrv find gives 1.88.0, set by icu_* 2.4 crates (url -> idna). Under the 1.90 limit.

Numbers in `PROGRESS.md`, section M0.
