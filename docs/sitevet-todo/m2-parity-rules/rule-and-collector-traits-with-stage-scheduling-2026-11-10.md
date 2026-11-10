---
id: "rule-and-collector-traits-with-stage-scheduling-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m2"]
order: 1
---

# Rule and collector traits with stage scheduling

`Rule`, `SiteRule`, `Collector`, `SiteCollector`, `RuleCx`, `Stage`. Run rules per stage, catch panics as info findings, `only` and `skip` filters.

## Requirements

- [ ] FR-RULE-02 [Must]: Every finding must carry rule ID, category, title, status (pass, fail, warn, info), severity (error, warning, info), message, and optional details, evidence (the offending markup, header or value) and fix text.
  Acceptance: the JSON schema test validates every finding from the fixture suite.
- [ ] FR-RULE-03 [Must]: One rule may emit many findings, each with its own ID override and severity (as the accessibility rule does for each axe violation).
  Acceptance: a fixture with 3 axe violations yields 3 accessibility findings with distinct IDs.
- [ ] FR-RULE-04 [Must]: Categories must be filterable with `only` and `skip` lists.
  Acceptance: `only = ["security"]` runs only security rules and collectors, and the report has no other categories.
- [ ] FR-EXT-01 [Must]: Public `Rule` and `Collector` traits. A caller can pass a custom list that replaces or extends the built-in registry.
  Acceptance: an example crate adds one custom rule and sees its findings in the report.

Spec: `docs/spec/06-build-spec.md`, section 8.
