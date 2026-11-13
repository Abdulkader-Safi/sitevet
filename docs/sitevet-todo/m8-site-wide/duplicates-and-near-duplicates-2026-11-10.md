---
id: "duplicates-and-near-duplicates-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T17:09:25.000Z"
labels: ["m8"]
order: 2
---

# Duplicates and near-duplicates

Hash-map groups, SimHash over 5-shingles.

## Requirements

- [x] FR-SITE-01 [Must]: Duplicate titles, meta descriptions and H1s across the crawl, each grouped with the URLs that share it.
  Acceptance: a fixture site with 3 pages sharing one title reports one group of 3.
- [x] FR-SITE-02 [Must]: Near-duplicate content detection across pages through a shingle hash, with a similarity percent per pair above a threshold.
  Acceptance: two fixture pages differing by one paragraph are flagged. Two unrelated pages are not.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- Duplicate titles, descriptions and H1s grouped with their URLs. The fixture's 3 shared titles give one group of 3.
- Near-duplicates with a 256-bit SimHash (64-bit was too noisy, see PROGRESS). A one-paragraph edit is flagged at 86%, an unrelated page is not.
