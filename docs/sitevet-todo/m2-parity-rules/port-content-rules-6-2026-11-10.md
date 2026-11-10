---
id: "port-content-rules-6-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T13:40:13.000Z"
labels: ["m2"]
order: 4
---

# Port content rules (6)

Port the 6 `content/*` rules from `src/rules/content.ts` in the TS package (commit `de27aea`) with the same IDs, severities and conditions. One file, one registry line per rule pattern. Pass and fail fixture tests for each rule.

## Requirements

- [x] FR-RULE-01 [Must]: All 94 rules in appendix A must be ported with the same rule IDs and severities as the TS package (and the same category for each), and the same conditions for each status.
  Acceptance: each rule has a fixture test in which it passes on a good fixture and fails on a bad one. The TS package's fixtures (`test/fixtures/*.html`) are ported and every expected result matches.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/src/rules/content.rs`, one unit struct per rule and one registry line each, with pass and fail tests in the same file. `content/keyword-stuffing` reads a new `text/` module: the main-content extraction, word and sentence tokens (unicode-segmentation, which is in the stack for M6), the English stop word list, the plural lemma and 1 to 3-word n-grams, all ported from `src/text.ts`. M6 extends this module rather than replacing it. Word count and text ratio use cheerio's `body.text()` semantics (script text included), and lengths count UTF-16 units like JavaScript.

The TS package's own expectations (`test/rules.test.ts`) are ported in `rules/parity.rs` and pass.
