---
id: "release-0-1-0-with-cargo-dist-2026-11-10"
status: "review"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:25:41.000Z"
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

## Result

Prepared, not published. `dist-workspace.toml` and `.github/workflows/release.yml` (cargo-dist 0.34): binaries for aarch64 and x86_64 macOS, aarch64 and x86_64 Linux (musl, static), x86_64 Windows, with shell and PowerShell installers. `dist plan` lists all five. The workflow runs only when a version tag is pushed.

Both crates have crates.io metadata and pass `cargo package`. A CI job (`lean-library`) fails if the library's dependency tree gains clap or rusqlite.

> [!warning]
> Publishing needs Safi's go: it cannot be undone. Steps: bump both crates to 0.1.0, `cargo login`, `cargo publish -p sitevet`, `cargo publish -p sitevet-cli`, then push the `v0.1.0` tag. After that, check `cargo install sitevet-cli` and the shell installer on a clean macOS and Ubuntu machine to tick FR-DIST-01 and FR-DIST-02.
