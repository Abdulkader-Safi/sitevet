---
id: "skipped-rule-reporting-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:46:04.000Z"
labels: ["m4"]
order: 4
---

# Skipped-rule reporting

Browser rules listed as skipped with a reason when neither browser nor PSI is on.

## Requirements

- [x] FR-RENDER-04 [Must]: Rules that need the rendered stage must be skipped, and listed as skipped with a reason, when neither browser nor PSI is enabled.
  Acceptance: a default audit lists the browser rules as "skipped: needs browser or psi_key", and they do not affect the score.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`report.skipped_rules` lists each rendered-stage rule with "needs browser or psi_key" when neither is on, or "the rendered stage stopped: <reason>" when Chrome died twice. Both reports show a "Skipped rules" table and the CLI prints "7 rules skipped (needs browser or psi_key)". Skipped rules never produce findings, so they cannot move the score (test in `tests/engine.rs`).
