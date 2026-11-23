---
id: "update-check-and-update-command-2026-11-23"
status: "review"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-23T14:40:00.000Z"
modified: "2026-11-23T14:40:00.000Z"
labels: ["0.7.0", "cli", "needs-review"]
order: 12
---

# Update check and sitevet update

For 0.7.0. Before an audit, offer a new release. `sitevet update` installs it at any time.

Done:

- [x] Spec: FR-CLI-08 and FR-CLI-09 in stage 03, NFR-03 allows one request a day to the GitHub releases API, stage 05 has the details
- [x] `sitevet audit` at a terminal asks `Update now? [y/N]` when a newer release is out. Yes installs it and reruns the same audit on the new binary. No, or any failure, audits on the current version
- [x] At most one request a day, cached in `update-check.json` next to `history.db`
- [x] Skipped when stdin or stderr is not a terminal, under `--quiet`, with `SITEVET_NO_UPDATE_CHECK`, or `update_check = false` under `[cli]`
- [x] Installs the way sitevet was installed: the release installer when its receipt points at the running binary, `cargo install` in Cargo's bin folder, otherwise it prints the command
- [x] Windows: the running binary is moved to `sitevet.exe.old` first and put back if the install fails
- [x] Tests against a local release API: up to date, a copy it cannot replace, no request without a terminal, the one-day cache
- [x] Checked by hand at a terminal: a 0.6.1 build updated itself to 0.6.2 through the real installer and reran the audit

> [!warning] Needs Safi
> Try the prompt in your own terminal. Not tried on Windows yet, only covered by CI. The wiki CLI pages still need the new command and key.
