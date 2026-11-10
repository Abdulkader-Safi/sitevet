---
id: "spawn-spike-from-a-non-tokio-executor-2026-11-10"
status: "done"
priority: "critical"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T09:42:25.000Z"
labels: ["m0"]
order: 3
---

# spawn() spike from a non-tokio executor

Owned 2-thread tokio runtime, a reqwest fetch loop sending events over async-channel, driven from `futures::executor::block_on` with no tokio context. Cancel, drop the handle, exit the process with no panic (`shutdown_background()` in `Drop`).

## Requirements

- [ ] FR-API-06 [Must]: The library must run inside an app that does not use tokio as its main executor (for example a GPUI app), without the caller managing a runtime. `spawn(url, options)` runs the audit on a runtime sitevet owns and returns a handle whose event stream, cancel and final report work from any executor.
  Acceptance: an example that calls `spawn` from a `futures::executor` (no tokio) receives every event and the final report for a fixture audit, and cancel stops it within 1 second.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

Owned 2-thread runtime driven from futures::executor::block_on with no tokio context. Full run delivers all 102 events and the report. Cancel returns the report in under 1 ms with 0 open connections. Dropping the handle mid-crawl uses shutdown_background(), takes under 1 ms, and panics 0 times. Passes on all three OSes. The FR-API-06 box stays open until M1 ships the real example against a fixture audit.

Numbers in `PROGRESS.md`, section M0.
