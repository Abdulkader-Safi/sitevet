---
id: "check-chromiumoxide-builds-for-musl-2026-11-10"
status: "done"
priority: "critical"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T09:42:25.000Z"
labels: ["m0"]
order: 2
---

# Check chromiumoxide builds for musl

Build the browser spike for `x86_64-unknown-linux-musl` with rustls. Record the result in `PROGRESS.md`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

Builds for x86_64-unknown-linux-musl with musl-tools on Ubuntu, reqwest rustls included. Output is a static-pie ELF, and that binary runs the whole browser spike with every check passing.

Numbers in `PROGRESS.md`, section M0.
