---
id: "rendered-crawl-option-2026-11-10"
status: "todo"
priority: "low"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m4"]
order: 5
---

# Rendered crawl option

`render_crawl` fetches pages through Chromium for link discovery. Should.

## Requirements

- [ ] FR-CRAWL-10 [Should]: With the browser feature on, a `render_crawl` option must fetch each page through Chromium, so links and content added by JavaScript are discovered and audited.
  Acceptance: on a fixture SPA whose links exist only after JS runs, `render_crawl = true` finds them and the default crawl does not.

Spec: `docs/spec/06-build-spec.md`, section 8.
