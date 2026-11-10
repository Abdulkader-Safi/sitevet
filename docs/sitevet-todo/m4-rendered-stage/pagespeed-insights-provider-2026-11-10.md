---
id: "pagespeed-insights-provider-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:46:04.000Z"
labels: ["m4"]
order: 3
---

# PageSpeed Insights provider

PSI v5 with recorded test responses, `psi_max_pages`, 4 in flight, one retry on 429.

## Requirements

- [x] FR-RENDER-03 [Must]: Without a local browser, a `psi_key` option must get Core Web Vitals and accessibility from the Google PageSpeed Insights API for the first `psi_max_pages` pages (default 5).
  Acceptance: with a recorded PSI response, the same rules fill in, and only 5 requests are made for a 20-page audit.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`render_stage/psi.rs`: PSI v5 with `category=performance&category=accessibility&strategy=mobile`, the first `psi_max_pages` pages, 4 in flight, 60 s timeout, one retry after `Retry-After` on a 429 (60 s at most), otherwise the page gets a "could not be rendered" result. The key comes from `psi_key` or `SITEVET_PSI_KEY` and never reaches the report, events or error text.

Test (`tests/psi.rs`) against a response in the PSI v5 shape (`tests/fixtures/psi/runPagespeed.json`, trimmed to the audits sitevet reads, since no key was available to record a live one): a 20-page audit sends 5 requests plus one retry after a 429, and the LCP, CLS, TTFB and accessibility rules fill in. The test points the library at the fixture with the `SITEVET_PSI_ENDPOINT` variable.
