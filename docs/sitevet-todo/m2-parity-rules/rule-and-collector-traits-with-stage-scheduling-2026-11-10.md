---
id: "rule-and-collector-traits-with-stage-scheduling-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:16:46.000Z"
labels: ["m2"]
order: 1
---

# Rule and collector traits with stage scheduling

`Rule`, `SiteRule`, `Collector`, `SiteCollector`, `RuleCx`, `Stage`. Run rules per stage, catch panics as info findings, `only` and `skip` filters.

## Requirements

- [x] FR-RULE-02 [Must]: Every finding must carry rule ID, category, title, status (pass, fail, warn, info), severity (error, warning, info), message, and optional details, evidence (the offending markup, header or value) and fix text.
  Acceptance: the JSON schema test validates every finding from the fixture suite.
- [x] FR-RULE-03 [Must]: One rule may emit many findings, each with its own ID override and severity (as the accessibility rule does for each axe violation).
  Acceptance: a fixture with 3 axe violations yields 3 accessibility findings with distinct IDs.
- [x] FR-RULE-04 [Must]: Categories must be filterable with `only` and `skip` lists.
  Acceptance: `only = ["security"]` runs only security rules and collectors, and the report has no other categories.
- [x] FR-EXT-01 [Must]: Public `Rule` and `Collector` traits. A caller can pass a custom list that replaces or extends the built-in registry.
  Acceptance: an example crate adds one custom rule and sees its findings in the report.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`dom_rule!`, `rule!` and `dom_collector!` in `rules/kit.rs` turn each rule into a unit struct implementing `Rule` with its stage. `RuleCx` gained `html` (raw markup, Dom stage) and `rendered` (`RenderedData`, Rendered stage), since a third of the TS rules read the raw HTML. `RuleResult::pass/fail/warn/info` and `with_evidence`, `MetricInput::with_status/with_note/table_capped` are small public builders, so custom rules read like built-in ones. Panic catching and stage scheduling came in M1.

FR-RULE-03: 3 axe violations give 3 findings with distinct IDs (`rules/accessibility.rs`). FR-RULE-04: `only = ["security"]` leaves only security findings, metrics and categories (`tests/engine.rs`). FR-EXT-01: `examples/custom-rule.rs`, and the engine tests pass custom rules through `Options.rules` and see their findings.

> [!warning]
> FR-RULE-02 needs the report JSON Schema, which M3 builds (FR-REPORT-01). Its box stays open until then. See PROGRESS.md.

FR-RULE-02 ticked in M3: `tests/report.rs` validates every finding from four fixture audits against the generated schema.
