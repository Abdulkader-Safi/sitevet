---
id: "sitevet-init-command-2026-11-10"
status: "todo"
priority: "low"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m3"]
order: 8
---

# sitevet init command

Commented `sitevet.toml` with every default. Refuses to overwrite without `--force`. Should.

## Requirements

- [ ] FR-CLI-07 [Should]: `sitevet init` writes a commented `sitevet.toml` with every key at its default.
  Acceptance: the written file parses back into default `Options`.

Spec: `docs/spec/06-build-spec.md`, section 8.
