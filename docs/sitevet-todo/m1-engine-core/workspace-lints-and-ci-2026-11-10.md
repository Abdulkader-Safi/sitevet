---
id: "workspace-lints-and-ci-2026-11-10"
status: "done"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T12:08:27.000Z"
labels: ["m1"]
order: 1
---

# Workspace, lints and CI

Create the workspace with `crates/sitevet` and `crates/sitevet-cli`, shared lints, `deny.toml`. GitHub Actions: fmt, clippy `-D warnings`, tests on macOS, Linux, Windows, `cargo deny`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

Workspace with `crates/sitevet` and `crates/sitevet-cli` (binary `sitevet`, a version stub until M3). `rust-version = "1.88"`, shared lints in `[workspace.lints]`, `clippy.toml`, `rustfmt.toml` (120 columns), MIT `LICENSE`.

`deny.toml` allows MIT, Apache-2.0, BSD-3, ISC, Unicode-3.0, Zlib, CDLA-Permissive-2.0 and MIT-0, with MPL-2.0 allowed only for scraper's CSS crates (cssparser, cssparser-macros, dtoa-short, selectors). `cargo deny check` passes locally.

`.github/workflows/ci.yml`: fmt, clippy with `-D warnings` and all features, tests on Linux, macOS and Windows, a 1.88 check job, cargo deny. The M0 spike workflow and `spikes/` are removed.
