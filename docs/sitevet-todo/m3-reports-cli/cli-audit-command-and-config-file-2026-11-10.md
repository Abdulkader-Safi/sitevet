---
id: "cli-audit-command-and-config-file-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:16:46.000Z"
labels: ["m3"]
order: 5
---

# CLI audit command and config file

One flag per option, `--format`, `--out`, `--config`, `sitevet.toml` with closest-key suggestions.

## Requirements

- [x] FR-CLI-01 [Must]: `sitevet audit <url>` runs an audit with flags for every `Options` field, plus `--format json|md|html`, `--out <path>` and `--config <path>`. Without `--out`, the report goes to stdout. Progress and logs go to stderr.
  Acceptance: `sitevet audit URL --format json > r.json` produces valid JSON, with nothing else in the file.
- [x] FR-CLI-02 [Must]: Read `sitevet.toml` from the current folder or `--config`. Flags override file values. Unknown keys are an error that names the key.
  Acceptance: a misspelled key fails with its name and the closest valid key.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`sitevet audit <url>` with one flag per `Options` field (boolean flags take `--flag` or `--flag=false`, lists are comma separated, `--psi-key` and `--chrome-path` also read `SITEVET_PSI_KEY` and `SITEVET_CHROME`), plus `--format`, `--out`, `--config`, `--fail-under`, `--fail-on`, `--quiet`, and global `--plain` and `--verbose`. The format defaults to the `--out` extension, else HTML.

`sitevet.toml` from `--config` or the current folder; flags win. Unknown keys, top level or in `[cli]`, fail with the key and the closest valid one: "invalid option `max_page`: unknown key, did you mean `max_pages`?". The suggestion lives in the library's `Options::from_toml_str`.

FR-API-04 is now complete: a unit test sets every field through TOML and through flags and gets equal Options. `--baseline` and `--no-save` arrive with history in M5.
