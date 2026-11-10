---
id: "crawler-queue-robots-and-sitemap-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T12:08:27.000Z"
labels: ["m1"]
order: 5
---

# Crawler: queue, robots and sitemap

BFS queue with `max_pages` and `max_depth`, URL normalisation, robots.txt obeyed with the blocked list, sitemap and sitemap index seeding, typed error for an unreachable start page.

## Requirements

- [x] FR-CRAWL-01 [Must]: The crawler must follow same-origin links from the start URL, breadth first, up to `max_pages` (default 100) and `max_depth` (default 5), fetching `concurrency` pages at once (default 5).
  Acceptance: on a fixture site with 150 linked pages, defaults crawl exactly 100 pages, none deeper than 5 clicks.
- [x] FR-CRAWL-03 [Must]: `robots.txt` and `sitemap.xml` (including sitemap indexes and sitemaps listed in robots.txt) must be fetched once per site and shared with every rule.
  Acceptance: a fixture server logs exactly one request each for `/robots.txt` and `/sitemap.xml` during a 20-page audit.
- [x] FR-CRAWL-04 [Must]: The crawler must obey `robots.txt` Disallow rules for its own user agent and for `*` by default. Skipped URLs appear in the report as "blocked by robots.txt". An `ignore_robots` option turns this off.
  Acceptance: a disallowed path is not requested by default, is listed as blocked, and is requested when `ignore_robots = true`.
- [x] FR-CRAWL-05 [Must]: The crawler must seed the queue from the sitemap as well as from links (phase 4), controlled by a `crawl_sitemap` option (default on).
  Acceptance: a page listed only in the sitemap and linked from nowhere is crawled.
- [x] FR-CRAWL-11 [Must]: If the start URL cannot be fetched (DNS failure, connection refused, non-HTML response), the audit must fail with a typed error. Errors on other pages become findings on that page and the audit continues.
  Acceptance: an unreachable start URL returns an error; one 500 page among 10 produces a report with 10 pages, one marked with its status.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crawl/mod.rs`: robots.txt and every sitemap it lists (or `/sitemap.xml`, sitemap indexes followed) are fetched once, before the crawl. The queue is breadth first: the start page, then sitemap URLs at depth 1, then links. Each round takes as many URLs as the page budget has left and fetches them `concurrency` at a time, in queue order. Fragments are dropped, and a page whose final URL was already audited is skipped, so a redirect never audits a page twice.

`crawl/robots.rs` obeys robots.txt through texting_robots for the `sitevet` token and `*`. Blocked URLs go to `report.blocked` with the best-matching `Disallow` line. `crawl/sitemap.rs` parses urlsets and indexes with quick-xml.

An unreachable start page is `Error::StartPageUnreachable`, a non-HTML one `Error::NotHtml`, a non-http URL `Error::InvalidUrl`. Other failures become a page with `status` and `error` set.

Tests cover all five requirements: 150 pages give exactly 100 within depth 5 (and 126 with a higher page cap), one request each for robots.txt and sitemap.xml, a disallowed path never requested and listed, then requested with `ignore_robots`, a sitemap-only page crawled, an unreachable start, and a 500 page among 10.
