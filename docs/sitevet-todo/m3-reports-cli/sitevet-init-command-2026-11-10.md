---
id: "sitevet-init-command-2026-11-10"
status: "done"
priority: "low"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:16:46.000Z"
labels: ["m3"]
order: 8
---

# sitevet init command

Commented `sitevet.toml` with every default. Refuses to overwrite without `--force`. Should.

## Requirements

- [x] FR-CLI-07 [Should]: `sitevet init` writes a commented `sitevet.toml` with every key at its default.
  Acceptance: the written file parses back into default `Options`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`sitevet init [--force]` writes a commented `sitevet.toml` with every key at its default (keys without a default, such as `user_agent` and `psi_key`, are shown commented out, with a note to prefer `SITEVET_PSI_KEY`). It refuses to overwrite without `--force`. Tests: the template parses back into default Options and mentions every key.
