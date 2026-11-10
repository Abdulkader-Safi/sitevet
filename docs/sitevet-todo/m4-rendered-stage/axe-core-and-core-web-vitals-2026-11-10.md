---
id: "axe-core-and-core-web-vitals-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m4"]
order: 2
---

# axe-core and Core Web Vitals

Inject the bundled axe-core, one finding per violation. Collect LCP, CLS, TTFB and page stats. Add `THIRD_PARTY.md` for axe-core.

## Requirements

- [ ] FR-RENDER-01 [Must]: With the browser option on, each page must be rendered in headless Chrome or Chromium found on the machine, and an axe-core accessibility pass must run, one finding per violation and one pass per passed check.
  Acceptance: on a fixture with a missing form label, an `accessibility/*` failure for `label` appears.
- [ ] FR-RENDER-02 [Must]: The rendered stage must collect Core Web Vitals and page stats: LCP, CLS, TTFB, DOM node count, page weight, request count.
  Acceptance: the `performance/lcp` and `performance/cls` rules report numbers for a rendered fixture.

Spec: `docs/spec/06-build-spec.md`, section 8.
