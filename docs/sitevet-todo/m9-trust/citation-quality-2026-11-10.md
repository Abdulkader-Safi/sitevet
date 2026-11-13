---
id: "citation-quality-2026-11-10"
status: "done"
priority: "low"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T17:23:22.000Z"
labels: ["m9"]
order: 5
---

# Citation quality

`data/authoritative-domains.txt`. Should.

## Requirements

- [x] FR-TRUST-04 [Should]: Citation quality: share of outbound citations pointing at a bundled list of recognised authoritative domains (government, education, major standards bodies, encyclopedias).
  Acceptance: a fixture citing `who.int` and a random blog reports 50%.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- `eeat/citation-quality` against `data/authoritative-domains.txt` and government and academic suffixes. `who.int` plus a blog gives 50%.
