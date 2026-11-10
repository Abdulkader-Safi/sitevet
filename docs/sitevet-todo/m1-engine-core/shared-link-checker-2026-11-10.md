---
id: "shared-link-checker-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m1"]
order: 6
---

# Shared link checker

One check per distinct URL per audit, through a shared future in a `Mutex<HashMap>`. HEAD first, GET on 405 or 501.

## Requirements

- [ ] FR-CRAWL-09 [Must]: A shared link checker must check each distinct URL once per audit, whichever rules ask about it.
  Acceptance: a URL linked from 10 pages is requested once.

Spec: `docs/spec/06-build-spec.md`, section 8.
