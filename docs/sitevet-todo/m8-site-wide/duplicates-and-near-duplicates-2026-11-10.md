---
id: "duplicates-and-near-duplicates-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m8"]
order: 2
---

# Duplicates and near-duplicates

Hash-map groups, SimHash over 5-shingles.

## Requirements

- [ ] FR-SITE-01 [Must]: Duplicate titles, meta descriptions and H1s across the crawl, each grouped with the URLs that share it.
  Acceptance: a fixture site with 3 pages sharing one title reports one group of 3.
- [ ] FR-SITE-02 [Must]: Near-duplicate content detection across pages through a shingle hash, with a similarity percent per pair above a threshold.
  Acceptance: two fixture pages differing by one paragraph are flagged. Two unrelated pages are not.

Spec: `docs/spec/06-build-spec.md`, section 8.
