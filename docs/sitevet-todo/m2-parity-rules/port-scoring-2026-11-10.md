---
id: "port-scoring-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T13:40:13.000Z"
labels: ["m2"]
order: 19
---

# Port scoring

`score_findings` and `compute_overall` with the TS test cases ported exactly.

## Requirements

- [x] FR-SCORE-01 [Must]: A health score from 0 to 100 per page, per category and overall, weighted by severity, using the same formula as the TS package so the same findings give the same score.
  Acceptance: the ported TS scoring tests produce identical numbers.
- [x] FR-SCORE-02 [Must]: Skipped rules, info findings and measurements never change the score.
  Acceptance: toggling collectors and skipped rules leaves the score unchanged.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`score_findings` and `compute_overall` were ported in M1. The TS `test/runner.test.ts` cases are now in `score.rs` with identical numbers (100, 0, 25, 89, 78), plus the warn factor and the floor at 0. FR-SCORE-02: scores are unchanged with collectors on and off, and with an info-only rule added (`tests/engine.rs`).
