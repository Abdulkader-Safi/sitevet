---
id: "shared-link-checker-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T12:08:27.000Z"
labels: ["m1"]
order: 6
---

# Shared link checker

One check per distinct URL per audit, through a shared future in a `Mutex<HashMap>`. HEAD first, GET on 405 or 501.

## Requirements

- [x] FR-CRAWL-09 [Must]: A shared link checker must check each distinct URL once per audit, whichever rules ask about it.
  Acceptance: a URL linked from 10 pages is requested once.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crawl/links.rs`: a `Mutex<HashMap<url, Shared<future>>>`, so concurrent callers await the same check. HEAD first, GET on 405 or 501. Every page the crawl fetches is seeded into the cache, so a crawled page is never requested again by the checker. Internal URLs that robots.txt disallows are not requested and get "blocked by robots.txt".

The checker runs before the Enriched stage when at least one Enriched rule or collector is selected. Test: a URL on another server linked from 10 pages is requested once, and a crawled page linked from all 10 is requested once in total.
