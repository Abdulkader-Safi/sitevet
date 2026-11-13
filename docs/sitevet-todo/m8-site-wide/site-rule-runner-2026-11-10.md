---
id: "site-rule-runner-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T17:09:25.000Z"
labels: ["m8"]
order: 1
---

# Site rule runner

Run site rules and collectors after the crawl in `spawn_blocking`. Site findings count in category scores and the overall penalty, scaled by affected pages.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- `site/` runs after the crawl in `spawn_blocking`. Site findings count once in category scores and add the TS overall weight scaled by the share of pages they touch. Unit test in `score.rs`.
