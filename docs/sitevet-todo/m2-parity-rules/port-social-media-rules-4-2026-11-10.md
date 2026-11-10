---
id: "port-social-media-rules-4-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T13:40:13.000Z"
labels: ["m2"]
order: 11
---

# Port social-media rules (4)

Port the 4 `social-media/*` rules from `src/rules/social-media.ts` in the TS package (commit `de27aea`) with the same IDs, severities and conditions. One file, one registry line per rule pattern. Pass and fail fixture tests for each rule.

## Requirements

- [x] FR-RULE-01 [Must]: All 94 rules in appendix A must be ported with the same rule IDs and severities as the TS package (and the same category for each), and the same conditions for each status.
  Acceptance: each rule has a fixture test in which it passes on a good fixture and fails on a bad one. The TS package's fixtures (`test/fixtures/*.html`) are ported and every expected result matches.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/src/rules/social_media.rs`, one unit struct per rule and one registry line each, with pass and fail tests in the same file. Same host list and comparisons as the TS rules.

The TS package's own expectations (`test/rules.test.ts`) are ported in `rules/parity.rs` and pass.
