---
id: "history-store-with-sqlite-2026-11-10"
status: "in-progress"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T18:29:12.000Z"
labels: ["m5"]
order: 1
---

# History store with SQLite

`history` feature, migrations with `user_version`, zstd blobs, `SITEVET_HOME` and OS data folder.

## Requirements

- [ ] FR-HIST-01 [Must]: Every CLI audit is saved to a local SQLite database in the OS data folder, unless `--no-save` is passed. The library offers the same store behind a `history` cargo feature, off by default.
  Acceptance: two runs create two rows. A run with `--no-save` creates none.

Spec: `docs/spec/06-build-spec.md`, section 8.
