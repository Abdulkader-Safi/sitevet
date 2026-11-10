---
id: "port-accessibility-rules-1-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T13:40:13.000Z"
labels: ["m2"]
order: 17
---

# Port accessibility rules (1)

Port the 1 `accessibility/*` rules from `src/rules/accessibility.ts` in the TS package (commit `de27aea`) with the same IDs, severities and conditions. One file, one registry line per rule pattern. Pass and fail fixture tests for each rule. Browser-stage logic now, it runs in M4.

## Requirements

- [x] FR-RULE-01 [Must]: All 94 rules in appendix A must be ported with the same rule IDs and severities as the TS package (and the same category for each), and the same conditions for each status.
  Acceptance: each rule has a fixture test in which it passes on a good fixture and fails on a bad one. The TS package's fixtures (`test/fixtures/*.html`) are ported and every expected result matches.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/src/rules/accessibility.rs`, one unit struct per rule and one registry line each, with pass and fail tests in the same file. Ported with its logic and tests now; it runs in M4 once the rendered stage exists. Each axe violation and each passed check becomes its own finding with an `accessibility/<axe id>` ID and its own severity (critical and serious are errors, moderate a warning, minor an info that shows as warn). The test feeds 3 violations and 1 pass through a synthetic `RenderedData` and gets 4 findings with distinct IDs (FR-RULE-03).

The TS package's own expectations (`test/rules.test.ts`) are ported in `rules/parity.rs` and pass.
