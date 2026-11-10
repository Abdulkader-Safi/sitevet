---
id: "pagedata-extraction-events-and-cancellation-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T12:08:27.000Z"
labels: ["m1"]
order: 7
---

# PageData extraction, events and cancellation

Extract PageData inside `spawn_blocking` and drop the DOM. Emit every event in order. One `CancellationToken` per audit.

## Requirements

- [x] FR-API-03 [Must]: Dropping the audit future or stream, or calling cancel on it, must stop all network and browser work.
  Acceptance: cancelling a 100-page audit mid-crawl leaves no open connections and no Chromium process after 1 second.
- [x] FR-EVENT-01 [Must]: Events: `AuditStart`, `PageFetched`, `CrawlComplete`, `RenderComplete`, `CheckComplete`, `PageComplete`, `AuditComplete`, with the same fields as the TS package events. Events after `CrawlComplete` carry progress against a known total.
  Acceptance: a fixture audit's event log matches the expected sequence and counts.
- [x] FR-EVENT-02 [Must]: Events must be `Send` and serialisable, so an app can forward them across threads or over a channel.
  Acceptance: a compile test sends events through a `std::sync::mpsc` channel and serialises one to JSON.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`extract/mod.rs` builds PageData inside one `spawn_blocking` call per page: title, `lang`, meta tags, canonical, headings, links (absolute URL, text, rel, internal flag), images, scripts, stylesheets and body text. The DOM stage rules and collectors run in the same blocking call, then the DOM is dropped. Fields for later phases (structured data, language detection, hashes, fingerprints) arrive with the milestone that needs them.

`engine/pipeline.rs` runs rules under `catch_unwind` (a panic is an info finding "rule crashed"), stamps findings with category, overrides, the 2,000-character evidence cap and an FNV-1a `key`, and scores with `score_findings` and `compute_overall`, ported from the TS package. Events follow the TS sequence and fields in snake_case.

One `CancellationToken` per audit wraps the whole pipeline in `select!`, so cancel or drop drops every in-flight request. Tests: the full event sequence and counts on the basic site, events through `std::sync::mpsc` and to JSON, and cancel, drop of the stream, and drop of the `audit` future on a slow 100-page site, each with zero open connections well within 1 s. The Chromium half of FR-API-03 is checked again in M4.
