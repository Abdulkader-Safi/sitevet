---
id: "fetcher-with-limits-redirects-and-ssrf-guard-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T12:08:27.000Z"
labels: ["m1"]
order: 4
---

# Fetcher with limits, redirects and SSRF guard

reqwest client with rustls, per-host semaphore and delay, timeouts, manual redirects with recorded hops, size caps on decoded bodies, private-address blocking when the start URL is public.

## Requirements

- [x] FR-CRAWL-06 [Must]: Per-request timeout (default 15 s), custom user agent, and an optional per-host delay between requests (default 0 ms) must be configurable.
  Acceptance: with `delay_ms = 500`, request start times to one host are at least 500 ms apart.
- [x] FR-CRAWL-07 [Must]: The default user agent must identify sitevet and its version and include the repository URL.
  Acceptance: the fixture server sees `sitevet/<version> (+https://github.com/Abdulkader-Safi/sitevet)`.
- [x] FR-CRAWL-08 [Must]: Redirects must be followed and recorded (chain length and final URL). URL normalisation must remove fragments and treat `/a` and `/a/` according to the server's redirect, so a page is never audited twice.
  Acceptance: `/a`, `/a/` and `/a#x` on a fixture site produce one audited page.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crawl/fetch.rs`: one reqwest client per audit with rustls, gzip, brotli, zstd and http2, redirects off. Up to 10 redirects followed by hand and recorded. Per-host semaphore (`concurrency`), per-host start spacing (`delay_ms`), a shared cap of 20 for external hosts, the timeout per request. Bodies are read decoded and cut at the cap (HTML 10 MB, robots.txt 500 KB, sitemaps 50 MB), so a compression bomb stops at the cap too. One retry after 1 s on a reset connection or a 502, 503 or 504. Debug log per request without the query string.

`crawl/ssrf.rs`: private, loopback, link-local (metadata), CGNAT and unique-local addresses are blocked when the start URL is public, checked in a custom DNS resolver after resolution (so rebinding is covered) and for literal IPs before the request.

Tests: delay spacing (FR-CRAWL-06), user agent (FR-CRAWL-07), `/a`, `/a/` and `/a#x` as one page with the redirect recorded (FR-CRAWL-08), private start URL refused when `allow_private_network = false`, 11 MB page truncated and flagged.
