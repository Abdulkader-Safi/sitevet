---
id: "local-seo-checks-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T17:23:22.000Z"
labels: ["m9"]
order: 4
---

# Local SEO checks

## Requirements

- [x] FR-TRUST-05 [Must]: Local SEO: LocalBusiness schema completeness (address, geo, opening hours), NAP consistency between page text and schema, and a Google Business Profile link.
  Acceptance: a fixture whose page phone differs from its schema phone reports a NAP mismatch.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- `eeat/nap-consistency` rule, LocalBusiness completeness table and Google Business Profile link. The phone mismatch fixture warns.
