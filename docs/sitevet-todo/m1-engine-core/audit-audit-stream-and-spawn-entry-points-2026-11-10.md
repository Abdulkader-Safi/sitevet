---
id: "audit-audit-stream-and-spawn-entry-points-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T12:08:27.000Z"
labels: ["m1"]
order: 8
---

# audit, audit_stream and spawn entry points

Public entry points, with `spawn` moved over from the M0 spike and the `spawn-futures` example.

## Requirements

- [x] FR-API-01 [Must]: The library must expose an async `audit(url, options)` that crawls, runs every enabled rule and collector, scores the result, and returns a typed `AuditReport`.
  Acceptance: given a local fixture server with 3 linked pages, `audit` returns a report with 3 pages, a score from 0 to 100, findings and metrics per page.
- [x] FR-API-02 [Must]: The library must expose `audit_stream(url, options)` that returns a stream of typed progress events, ending with an event that carries the final report.
  Acceptance: the stream yields `AuditStart`, then at least one `PageFetched`, then `CrawlComplete`, then `CheckComplete` and `PageComplete` events, then `AuditComplete` with a report equal to what `audit` returns for the same input.
- [x] FR-API-06 [Must]: The library must run inside an app that does not use tokio as its main executor (for example a GPUI app), without the caller managing a runtime. `spawn(url, options)` runs the audit on a runtime sitevet owns and returns a handle whose event stream, cancel and final report work from any executor.
  Acceptance: an example that calls `spawn` from a `futures::executor` (no tokio) receives every event and the final report for a fixture audit, and cancel stops it within 1 second.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`audit()`, `audit_stream()` (a `Stream` with `cancel()` and `final_report()`, cancels on drop) and `spawn()` returning `AuditHandle` (`events()`, `cancel()`, `report()`, cancels on drop). `spawn` uses one lazily created runtime with `runtime_threads` workers, held through a `Weak`, and shut down with `shutdown_background()` when the last handle drops. Moved over from the M0 spike.

`report()` drops the handle's own event receiver first, so an app that never reads events cannot stall the audit on the bounded channel (capacity 1,024).

`examples/spawn-futures.rs` drives `spawn` from `futures::executor`. `tests/spawn.rs` does the same against a fixture server on another thread, with no tokio context on the test thread: every event and the report arrive, and cancel and drop finish within 1 s with no open connections.

FR-API-01 is ticked with test rules and a test collector passed through `Options.rules` and `Options.collectors`. The built-in rules land in M2.
