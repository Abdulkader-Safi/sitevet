---
id: "structured-data-trust-signals-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T17:23:22.000Z"
labels: ["m9"]
order: 2
---

# Structured data trust signals

## Requirements

- [x] FR-TRUST-02 [Must]: Structured data trust signals listed by name: which of these are present: Article, BlogPosting, Organization, author, datePublished, dateModified.
  Acceptance: fixture table matches.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- `eeat/schema-trust-signals` table for the six names, read from the JSON-LD, microdata and RDFa tree. Organization subtypes count.
