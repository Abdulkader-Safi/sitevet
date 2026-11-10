---
id: "options-struct-validation-and-toml-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T12:08:27.000Z"
labels: ["m1"]
order: 2
---

# Options struct, validation and TOML

`Options` with every field and default from stage 5, `validate()`, `from_toml_str()` with unknown-key errors.

## Requirements

- [ ] FR-API-04 [Must]: All options must live in one public `Options` struct with documented defaults. The CLI flags and the `sitevet.toml` keys map one to one onto its fields.
  Acceptance: a test sets every field through TOML and through flags and gets equal `Options` values.
- [x] FR-CRAWL-02 [Must]: `max_pages` must be capped at 10,000. Higher values are rejected with a clear error.
  Acceptance: `max_pages = 10001` fails validation with a message naming the cap.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`Options` has every stage 5 field and default. `validate()` rejects `max_pages` outside 1 to 10,000 with a message naming the cap, zero concurrency, timeout or runtime threads, `browser` and `media` without their cargo features, and `screenshots` or `render_crawl` without `browser`. `from_toml_str()` ignores the `[cli]` table and rejects unknown keys by name.

`psi_key` is read from TOML but never serialised, and `Debug` prints the serialised form, so it cannot reach a log.

FR-API-04 is half done: TOML sets every field (unit test). The flags half needs the CLI, so the box is ticked in M3. The closest-key hint for typos is FR-CLI-02, also M3.

`allow_private_network` is `Option<bool>`: unset means "decide from the start URL", which is the spec's dynamic default.
