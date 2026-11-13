---
id: "schema-tree-with-microdata-and-rdfa-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T13:55:21.000Z"
labels: ["m7"]
order: 5
---

# Schema tree with microdata and RDFa

## Requirements

- [x] FR-AEO-06 [Must]: Schema depth: a nested type tree of all structured data with a presence status per node, read from JSON-LD, microdata and RDFa. FAQPage and HowTo validation when present.
  Acceptance: a microdata fixture produces the same tree as its JSON-LD twin.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- One tree from JSON-LD, microdata and RDFa (`src/schema.rs`), shown as `structured-data/schema-tree`. Microdata and RDFa twins match JSON-LD.
- New rules `structured-data/faqpage-valid` and `structured-data/howto-valid`.
