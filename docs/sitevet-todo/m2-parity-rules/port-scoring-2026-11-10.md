---
id: "port-scoring-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m2"]
order: 19
---

# Port scoring

`score_findings` and `compute_overall` with the TS test cases ported exactly.

## Requirements

- [ ] FR-SCORE-01 [Must]: A health score from 0 to 100 per page, per category and overall, weighted by severity, using the same formula as the TS package so the same findings give the same score.
  Acceptance: the ported TS scoring tests produce identical numbers.
- [ ] FR-SCORE-02 [Must]: Skipped rules, info findings and measurements never change the score.
  Acceptance: toggling collectors and skipped rules leaves the score unchanged.

Spec: `docs/spec/06-build-spec.md`, section 8.
