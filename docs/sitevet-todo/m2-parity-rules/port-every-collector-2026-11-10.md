---
id: "port-every-collector-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T13:40:13.000Z"
labels: ["m2"]
order: 18
---

# Port every collector

Port `src/metrics/*.ts` collectors with the 25-row table cap and `total`. Fixture test per collector, and a test that the score is unchanged with collectors on and off.

## Requirements

- [x] FR-MET-01 [Must]: Every collector in the TS package (`src/metrics/*.ts`, IDs in appendix B) must be ported. Metrics carry a value or a table plus a status of ok, warn, bad or missing, and never change the score.
  Acceptance: each collector has a fixture test. A test confirms the score is identical with collectors on and off.
- [x] FR-MET-02 [Must]: Table metrics must be capped at a row limit with a "showing N of M" total.
  Acceptance: a fixture with 300 links gives a link table of at most the cap and a total of 300.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/src/metrics/`: page, markup, directives, links, headings and meta, mirroring the TS files, 14 collectors. Tables cap at 25 rows (40 for the heading outline, as in the TS package) and keep `total`. Every assertion in the TS `test/metrics.test.ts` is ported in `rules/parity.rs`, plus a 300-link page giving 25 rows and a total of 300 (FR-MET-02). `tests/engine.rs` audits the basic site with all collectors and with none: every page, category and overall score is identical (FR-MET-01).

Two text changes from the TS output, for sitevet's writing rules: empty cells and the outline indent use "-" instead of an em dash, and `clip` ends with "..." instead of the ellipsis character.

The phase 2 text collectors (`content/text-statistics`, `content/readability`, `content/keywords`) are FR-TEXT-04 to FR-TEXT-07 and land with the text engine in M6.
