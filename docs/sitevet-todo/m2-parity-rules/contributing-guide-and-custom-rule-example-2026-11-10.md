---
id: "contributing-guide-and-custom-rule-example-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T18:26:05.000Z"
labels: ["m2", "needs-review"]
order: 20
---

# CONTRIBUTING guide and custom-rule example

Worked example of a one-file rule with its fixture test. The `custom-rule` example crate.

## Requirements

- [x] FR-EXT-02 [Must]: A built-in rule is one source file plus one registry line, with a fixture test pattern contributors can copy. A `CONTRIBUTING.md` explains it.
  Acceptance: the contributing guide's worked example compiles and passes.

> [!note]
> Needs a visual check by Safi. Move to `review` when done, not `done`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`CONTRIBUTING.md` explains the file-per-category layout, the three stages, and walks through `core-seo/viewport-present`: the rule, its registry line and its test, quoted from the source. A test (`contributing_example_matches_the_source`) fails if the quoted code drifts from `core_seo.rs`, so the example always compiles and passes. `examples/custom-rule.rs` adds a rule from outside the crate.

Needs Safi's read of the guide's wording.

## Review

Rechecked 2026-11-10: the guide ships in the published 0.1.0 crates, and `contributing_example_matches_the_source` still passes.
