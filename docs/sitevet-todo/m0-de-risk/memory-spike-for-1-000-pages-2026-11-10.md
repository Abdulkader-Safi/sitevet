---
id: "memory-spike-for-1-000-pages-2026-11-10"
status: "done"
priority: "critical"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T09:42:25.000Z"
labels: ["m0"]
order: 4
---

# Memory spike for 1,000 pages

Parse 1,000 generated fixture pages through `spawn_blocking`, extract a PageData-like struct, drop the DOM, measure peak resident memory. Target: projected under 500 MB (NFR-02).

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

Peak RSS for 1,000 pages (172 MB of HTML, one 10 MB page) is 306 to 330 MB on Linux and 393 to 434 MB on macOS, under the 500 MB target. Windows not measured (no getrusage). Projection for 10,000 pages is about 2 GB: raised as an open item for Safi.

Numbers in `PROGRESS.md`, section M0.
