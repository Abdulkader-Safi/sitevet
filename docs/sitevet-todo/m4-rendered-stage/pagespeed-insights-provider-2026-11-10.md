---
id: "pagespeed-insights-provider-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m4"]
order: 3
---

# PageSpeed Insights provider

PSI v5 with recorded test responses, `psi_max_pages`, 4 in flight, one retry on 429.

## Requirements

- [ ] FR-RENDER-03 [Must]: Without a local browser, a `psi_key` option must get Core Web Vitals and accessibility from the Google PageSpeed Insights API for the first `psi_max_pages` pages (default 5).
  Acceptance: with a recorded PSI response, the same rules fill in, and only 5 requests are made for a 20-page audit.

Spec: `docs/spec/06-build-spec.md`, section 8.
