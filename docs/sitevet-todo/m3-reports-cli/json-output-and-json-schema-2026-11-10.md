---
id: "json-output-and-json-schema-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:16:46.000Z"
labels: ["m3"]
order: 1
---

# JSON output and JSON Schema

Report JSON with `schema_version`, generated `schema/report.schema.json`, and the test that every fixture report validates and the committed schema is current.

## Requirements

- [x] FR-REPORT-01 [Must]: JSON output: the full report with a `schema_version` field, documented in a published JSON Schema file.
  Acceptance: every fixture report validates against the schema.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`render(&report, Format::Json)` is pretty JSON with `schema_version: 1`. `report_schema()` generates a draft 2020-12 JSON Schema with schemars, in serialise mode, so `psi_key` is absent. The committed copy is `schema/report.schema.json`; a test fails when it drifts (`UPDATE_SCHEMA=1` rewrites it).

`tests/report.rs` audits four fixture sites (basic, Arabic, all 8 TS fixture pages as one site, and a site with a 500 page) with every rule, and validates each report with jsonschema: over 1,000 findings checked. That also covers FR-RULE-02 from M2.
