---
id: "port-url-structure-rules-8-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T13:40:13.000Z"
labels: ["m2"]
order: 10
---

# Port url-structure rules (8)

Port the 8 `url-structure/*` rules from `src/rules/url-structure.ts` in the TS package (commit `de27aea`) with the same IDs, severities and conditions. One file, one registry line per rule pattern. Pass and fail fixture tests for each rule.

## Requirements

- [x] FR-RULE-01 [Must]: All 94 rules in appendix A must be ported with the same rule IDs and severities as the TS package (and the same category for each), and the same conditions for each status.
  Acceptance: each rule has a fixture test in which it passes on a good fixture and fails on a bad one. The TS package's fixtures (`test/fixtures/*.html`) are ported and every expected result matches.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/src/rules/url_structure.rs`, one unit struct per rule and one registry line each, with pass and fail tests in the same file. A port of JavaScript's `decodeURI` (reserved characters stay encoded, a malformed escape means "Could not parse URL"), and `url.search` and `searchParams` semantics through the url crate.

The TS package's own expectations (`test/rules.test.ts`) are ported in `rules/parity.rs` and pass.
