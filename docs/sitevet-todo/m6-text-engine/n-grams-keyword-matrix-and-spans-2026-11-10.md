---
id: "n-grams-keyword-matrix-and-spans-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T13:17:08.000Z"
labels: ["m6"]
order: 4
---

# N-grams, keyword matrix and spans

1 to 3-word phrases, stop word table, relevance matrix, byte-range spans in headings.

## Requirements

- [x] FR-TEXT-06 [Must]: Top phrases as 1, 2 and 3-word n-grams with counts and percentages, plus a stop word table.
  Acceptance: a fixture with a known repeated phrase lists it first among 2-word phrases.
- [x] FR-TEXT-07 [Must]: Keyword relevance matrix: each of the top 10 keywords present or absent in title, meta description and H1. Plus highlight spans for those keywords in every heading.
  Acceptance: spans in the fixture point at the exact byte ranges of the keyword.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- `content/keywords` ported: 1 to 3-word phrase tables, the stop word table, the relevance matrix and heading highlights.
- Spans are byte ranges: `0-6 7-11` for "Garden soil" in the fixture title.
