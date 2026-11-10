---
id: "spawn-spike-from-a-non-tokio-executor-2026-11-10"
status: "todo"
priority: "critical"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m0"]
order: 3
---

# spawn() spike from a non-tokio executor

Owned 2-thread tokio runtime, a reqwest fetch loop sending events over async-channel, driven from `futures::executor::block_on` with no tokio context. Cancel, drop the handle, exit the process with no panic (`shutdown_background()` in `Drop`).

## Requirements

- [ ] FR-API-06 [Must]: The library must run inside an app that does not use tokio as its main executor (for example a GPUI app), without the caller managing a runtime. `spawn(url, options)` runs the audit on a runtime sitevet owns and returns a handle whose event stream, cancel and final report work from any executor.
  Acceptance: an example that calls `spawn` from a `futures::executor` (no tokio) receives every event and the final report for a fixture audit, and cancel stops it within 1 second.

Spec: `docs/spec/06-build-spec.md`, section 8.
