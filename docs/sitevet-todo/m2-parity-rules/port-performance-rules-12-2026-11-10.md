---
id: "port-performance-rules-12-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m2"]
order: 16
---

# Port performance rules (12)

Port the 12 `performance/*` rules from `src/rules/performance.ts` in the TS package (commit `de27aea`) with the same IDs, severities and conditions. One file, one registry line per rule pattern. Pass and fail fixture tests for each rule. Browser-stage logic now, it runs in M4.

## Requirements

- [ ] FR-RULE-01 [Must]: All 94 rules in appendix A must be ported with the same rule IDs and severities as the TS package (and the same category for each), and the same conditions for each status.
  Acceptance: each rule has a fixture test in which it passes on a good fixture and fails on a bad one. The TS package's fixtures (`test/fixtures/*.html`) are ported and every expected result matches.

Spec: `docs/spec/06-build-spec.md`, section 8.
