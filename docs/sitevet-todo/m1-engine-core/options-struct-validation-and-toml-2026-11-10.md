---
id: "options-struct-validation-and-toml-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m1"]
order: 2
---

# Options struct, validation and TOML

`Options` with every field and default from stage 5, `validate()`, `from_toml_str()` with unknown-key errors.

## Requirements

- [ ] FR-API-04 [Must]: All options must live in one public `Options` struct with documented defaults. The CLI flags and the `sitevet.toml` keys map one to one onto its fields.
  Acceptance: a test sets every field through TOML and through flags and gets equal `Options` values.
- [ ] FR-CRAWL-02 [Must]: `max_pages` must be capped at 10,000. Higher values are rejected with a clear error.
  Acceptance: `max_pages = 10001` fails validation with a message naming the cap.

Spec: `docs/spec/06-build-spec.md`, section 8.
