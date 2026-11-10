---
id: "fixture-server-and-first-fixture-sites-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T12:08:27.000Z"
labels: ["m1"]
order: 3
---

# Fixture server and first fixture sites

`tests/support` server on wiremock serving `tests/fixtures/sites/<name>/` with a `site.toml` for headers, statuses and redirects.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/tests/support/mod.rs`: one wiremock responder serves a `Site` (a path to route map). `Site::from_dir("basic")` loads `tests/fixtures/sites/basic/` plus `site.toml` overrides (status, headers, body, delay_ms). Bodies can use `{{origin}}`, replaced with the server URL, so sitemaps and robots files can hold absolute URLs on a random port.

Every request is logged with method, path, arrival time and user agent. `counting_proxy` counts open client connections for the cancel tests. `ThreadFixture` runs the server on its own thread and runtime for the no-tokio `spawn` test. `chain_site(n, delay)` generates linked pages.

First fixture site `basic`: 4 pages, a page only the sitemap lists, a path robots.txt disallows, a broken link and a page without a title.
