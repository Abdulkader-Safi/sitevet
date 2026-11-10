---
id: "rendered-crawl-option-2026-11-10"
status: "done"
priority: "low"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:46:04.000Z"
labels: ["m4"]
order: 5
---

# Rendered crawl option

`render_crawl` fetches pages through Chromium for link discovery. Should.

## Requirements

- [x] FR-CRAWL-10 [Should]: With the browser feature on, a `render_crawl` option must fetch each page through Chromium, so links and content added by JavaScript are discovered and audited.
  Acceptance: on a fixture SPA whose links exist only after JS runs, `render_crawl = true` finds them and the default crawl does not.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

With `render_crawl = true` (needs `browser`), each crawled page is also loaded in Chrome and the DOM after scripts ran replaces the fetched HTML for link discovery and the DOM rules. Status and headers still come from the HTTP fetch. Test: a page whose only link is added by a script is found with `render_crawl` and missed without it.
