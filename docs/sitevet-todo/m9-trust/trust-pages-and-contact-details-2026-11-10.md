---
id: "trust-pages-and-contact-details-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T17:23:22.000Z"
labels: ["m9"]
order: 3
---

# Trust pages and contact details

## Requirements

- [x] FR-TRUST-03 [Must]: Trust pages and details: refund or returns page, phone number and physical address, ads or affiliate disclosure, editorial policy page. Detection uses link text and URL patterns in English plus common equivalents in the page language where the libraries allow.
  Acceptance: an English fixture with a `/refund-policy` link passes the refund check.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- Rules `legal/refund-policy`, `legal/ad-disclosure`, `eeat/contact-details` and `eeat/editorial-policy`, English plus common words in six more languages. `/refund-policy` passes.
