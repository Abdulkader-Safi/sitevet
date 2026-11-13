---
id: "author-and-dates-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T17:23:22.000Z"
labels: ["m9"]
order: 1
---

# Author and dates

## Requirements

- [x] FR-TRUST-01 [Must]: Author box or byline detected as a unit, author page link, short author bio, and the actual publish and updated dates as values.
  Acceptance: the article fixture reports author name, link, bio present and both dates.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- Byline read as a unit, author name, author page link, bio, and both dates as values (`eeat/author-*`, `eeat/date-*`). The fixture reports all five.
