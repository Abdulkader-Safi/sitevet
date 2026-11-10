---
id: "history-store-with-sqlite-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T18:35:12.000Z"
labels: ["m5"]
order: 1
---

# History store with SQLite

`history` feature, migrations with `user_version`, zstd blobs, `SITEVET_HOME` and OS data folder.

## Requirements

- [x] FR-HIST-01 [Must]: Every CLI audit is saved to a local SQLite database in the OS data folder, unless `--no-save` is passed. The library offers the same store behind a `history` cargo feature, off by default.
  Acceptance: two runs create two rows. A run with `--no-save` creates none.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/src/history/mod.rs` behind the `history` feature (rusqlite 0.41 bundled, zstd 0.14, directories 6). `history.db` sits under `SITEVET_HOME` or the OS data folder. Table `runs` as in stage 5, migrations tracked with `PRAGMA user_version`, a database from a newer sitevet opens read-only with a warning, and a run from a newer report schema shows its summary only. API: `History::open`, `save`, `list`, `summary`, `load`, `delete`, `prune`, `latest_for`.

The CLI saves every audit and prints "saved as run N"; `--no-save` or `save = false` under `[cli]` skips it, and a failed save warns without changing the exit code. Test: two runs make two rows, a `--no-save` run none. rusqlite 0.41 needs Rust 1.89, so the MSRV moved from 1.88 to 1.89 (the spec allows up to 1.90).
