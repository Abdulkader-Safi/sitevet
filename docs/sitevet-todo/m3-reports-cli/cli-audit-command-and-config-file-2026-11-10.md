---
id: "cli-audit-command-and-config-file-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m3"]
order: 5
---

# CLI audit command and config file

One flag per option, `--format`, `--out`, `--config`, `sitevet.toml` with closest-key suggestions.

## Requirements

- [ ] FR-CLI-01 [Must]: `sitevet audit <url>` runs an audit with flags for every `Options` field, plus `--format json|md|html`, `--out <path>` and `--config <path>`. Without `--out`, the report goes to stdout. Progress and logs go to stderr.
  Acceptance: `sitevet audit URL --format json > r.json` produces valid JSON, with nothing else in the file.
- [ ] FR-CLI-02 [Must]: Read `sitevet.toml` from the current folder or `--config`. Flags override file values. Unknown keys are an error that names the key.
  Acceptance: a misspelled key fails with its name and the closest valid key.

Spec: `docs/spec/06-build-spec.md`, section 8.
