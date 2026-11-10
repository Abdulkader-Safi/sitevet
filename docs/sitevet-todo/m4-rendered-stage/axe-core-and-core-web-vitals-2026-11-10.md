---
id: "axe-core-and-core-web-vitals-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:46:04.000Z"
labels: ["m4"]
order: 2
---

# axe-core and Core Web Vitals

Inject the bundled axe-core, one finding per violation. Collect LCP, CLS, TTFB and page stats. Add `THIRD_PARTY.md` for axe-core.

## Requirements

- [x] FR-RENDER-01 [Must]: With the browser option on, each page must be rendered in headless Chrome or Chromium found on the machine, and an axe-core accessibility pass must run, one finding per violation and one pass per passed check.
  Acceptance: on a fixture with a missing form label, an `accessibility/*` failure for `label` appears.
- [x] FR-RENDER-02 [Must]: The rendered stage must collect Core Web Vitals and page stats: LCP, CLS, TTFB, DOM node count, page weight, request count.
  Acceptance: the `performance/lcp` and `performance/cls` rules report numbers for a rendered fixture.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

axe-core 4.14.0 bundled unmodified in `crates/sitevet/assets/axe.min.js` with its licence files, listed in `THIRD_PARTY.md` (MPL-2.0, SHA-256 recorded). Per page: open `about:blank`, bring to front, navigate (the M0 order), wait for the largest contentful paint plus 1 s for layout shifts (5 s at most), read navigation and resource timing, then run axe with its default rule set as the TS package's AxeBuilder did. Each violation becomes an `accessibility/<id>` finding, each passed check a pass.

Tests with `SITEVET_TEST_CHROME=1` (`tests/browser.rs`): a page with an unlabelled input gets `accessibility/label` failing; `performance/lcp`, `cls` and `dom-size` report numbers. Live: 6 pages of abdulkadersafi.com rendered in 18 s, LCP 1.04 s, CLS 0.005.
