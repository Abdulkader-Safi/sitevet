---
id: "port-every-collector-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m2"]
order: 18
---

# Port every collector

Port `src/metrics/*.ts` collectors with the 25-row table cap and `total`. Fixture test per collector, and a test that the score is unchanged with collectors on and off.

## Requirements

- [ ] FR-MET-01 [Must]: Every collector in the TS package (`src/metrics/*.ts`, IDs in appendix B) must be ported. Metrics carry a value or a table plus a status of ok, warn, bad or missing, and never change the score.
  Acceptance: each collector has a fixture test. A test confirms the score is identical with collectors on and off.
- [ ] FR-MET-02 [Must]: Table metrics must be capped at a row limit with a "showing N of M" total.
  Acceptance: a fixture with 300 links gives a link table of at most the cap and a total of 300.

Spec: `docs/spec/06-build-spec.md`, section 8.
