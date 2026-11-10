---
id: "pagedata-extraction-events-and-cancellation-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m1"]
order: 7
---

# PageData extraction, events and cancellation

Extract PageData inside `spawn_blocking` and drop the DOM. Emit every event in order. One `CancellationToken` per audit.

## Requirements

- [ ] FR-API-03 [Must]: Dropping the audit future or stream, or calling cancel on it, must stop all network and browser work.
  Acceptance: cancelling a 100-page audit mid-crawl leaves no open connections and no Chromium process after 1 second.
- [ ] FR-EVENT-01 [Must]: Events: `AuditStart`, `PageFetched`, `CrawlComplete`, `RenderComplete`, `CheckComplete`, `PageComplete`, `AuditComplete`, with the same fields as the TS package events. Events after `CrawlComplete` carry progress against a known total.
  Acceptance: a fixture audit's event log matches the expected sequence and counts.
- [ ] FR-EVENT-02 [Must]: Events must be `Send` and serialisable, so an app can forward them across threads or over a channel.
  Acceptance: a compile test sends events through a `std::sync::mpsc` channel and serialises one to JSON.

Spec: `docs/spec/06-build-spec.md`, section 8.
