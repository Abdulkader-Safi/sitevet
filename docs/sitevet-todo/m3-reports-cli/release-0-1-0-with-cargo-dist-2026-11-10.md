---
id: "release-0-1-0-with-cargo-dist-2026-11-10"
status: "todo"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m3", "needs-review"]
order: 9
---

# Release 0.1.0 with cargo-dist

cargo-dist config, publish both crates, GitHub Release with binaries for 5 targets and installers. Update the README to say what works.

## Requirements

- [ ] FR-DIST-01 [Must]: Publish the library crate `sitevet` and the CLI crate `sitevet-cli` (binary name `sitevet`) to crates.io. `cargo install sitevet-cli` installs the CLI.
  Acceptance: a clean machine runs `cargo install sitevet-cli` and then `sitevet --version`. A project with `sitevet` as a dependency compiles without clap or rusqlite in `cargo tree`.
- [ ] FR-DIST-02 [Must]: Prebuilt CLI binaries on GitHub Releases for macOS arm64 and x86_64, Linux x86_64 and arm64 (static), Windows x86_64, plus a one-line shell installer.
  Acceptance: the installer works on a fresh macOS and a fresh Ubuntu machine.

> [!note]
> Needs a visual check by Safi. Move to `review` when done, not `done`.

Spec: `docs/spec/06-build-spec.md`, section 8.
