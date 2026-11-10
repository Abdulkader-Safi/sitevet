---
id: "json-output-and-json-schema-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m3"]
order: 1
---

# JSON output and JSON Schema

Report JSON with `schema_version`, generated `schema/report.schema.json`, and the test that every fixture report validates and the committed schema is current.

## Requirements

- [ ] FR-REPORT-01 [Must]: JSON output: the full report with a `schema_version` field, documented in a published JSON Schema file.
  Acceptance: every fixture report validates against the schema.

Spec: `docs/spec/06-build-spec.md`, section 8.
