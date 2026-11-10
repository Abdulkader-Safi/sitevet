---
id: "port-internationalization-rules-2-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T13:40:13.000Z"
labels: ["m2"]
order: 12
---

# Port internationalization rules (2)

Port the 2 `internationalization/*` rules from `src/rules/internationalization.ts` in the TS package (commit `de27aea`) with the same IDs, severities and conditions. One file, one registry line per rule pattern. Pass and fail fixture tests for each rule.

## Requirements

- [x] FR-RULE-01 [Must]: All 94 rules in appendix A must be ported with the same rule IDs and severities as the TS package (and the same category for each), and the same conditions for each status.
  Acceptance: each rule has a fixture test in which it passes on a good fixture and fails on a bad one. The TS package's fixtures (`test/fixtures/*.html`) are ported and every expected result matches.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/src/rules/internationalization.rs`, one unit struct per rule and one registry line each, with pass and fail tests in the same file. Tested on the TS `directives.html` fixture and hand-made invalid sets.

The TS package's own expectations (`test/rules.test.ts`) are ported in `rules/parity.rs` and pass.
