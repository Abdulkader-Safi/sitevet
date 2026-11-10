---
id: "audit-audit-stream-and-spawn-entry-points-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m1"]
order: 8
---

# audit, audit_stream and spawn entry points

Public entry points, with `spawn` moved over from the M0 spike and the `spawn-futures` example.

## Requirements

- [ ] FR-API-01 [Must]: The library must expose an async `audit(url, options)` that crawls, runs every enabled rule and collector, scores the result, and returns a typed `AuditReport`.
  Acceptance: given a local fixture server with 3 linked pages, `audit` returns a report with 3 pages, a score from 0 to 100, findings and metrics per page.
- [ ] FR-API-02 [Must]: The library must expose `audit_stream(url, options)` that returns a stream of typed progress events, ending with an event that carries the final report.
  Acceptance: the stream yields `AuditStart`, then at least one `PageFetched`, then `CrawlComplete`, then `CheckComplete` and `PageComplete` events, then `AuditComplete` with a report equal to what `audit` returns for the same input.
- [ ] FR-API-06 [Must]: The library must run inside an app that does not use tokio as its main executor (for example a GPUI app), without the caller managing a runtime. `spawn(url, options)` runs the audit on a runtime sitevet owns and returns a handle whose event stream, cancel and final report work from any executor.
  Acceptance: an example that calls `spawn` from a `futures::executor` (no tokio) receives every event and the final report for a fixture audit, and cancel stops it within 1 second.

Spec: `docs/spec/06-build-spec.md`, section 8.
