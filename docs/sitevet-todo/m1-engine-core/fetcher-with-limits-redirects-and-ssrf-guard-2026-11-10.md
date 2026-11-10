---
id: "fetcher-with-limits-redirects-and-ssrf-guard-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m1"]
order: 4
---

# Fetcher with limits, redirects and SSRF guard

reqwest client with rustls, per-host semaphore and delay, timeouts, manual redirects with recorded hops, size caps on decoded bodies, private-address blocking when the start URL is public.

## Requirements

- [ ] FR-CRAWL-06 [Must]: Per-request timeout (default 15 s), custom user agent, and an optional per-host delay between requests (default 0 ms) must be configurable.
  Acceptance: with `delay_ms = 500`, request start times to one host are at least 500 ms apart.
- [ ] FR-CRAWL-07 [Must]: The default user agent must identify sitevet and its version and include the repository URL.
  Acceptance: the fixture server sees `sitevet/<version> (+https://github.com/Abdulkader-Safi/sitevet)`.
- [ ] FR-CRAWL-08 [Must]: Redirects must be followed and recorded (chain length and final URL). URL normalisation must remove fragments and treat `/a` and `/a/` according to the server's redirect, so a page is never audited twice.
  Acceptance: `/a`, `/a/` and `/a#x` on a fixture site produce one audited page.

Spec: `docs/spec/06-build-spec.md`, section 8.
