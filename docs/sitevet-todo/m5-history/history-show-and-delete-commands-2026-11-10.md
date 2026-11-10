---
id: "history-show-and-delete-commands-2026-11-10"
status: "in-progress"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T18:29:12.000Z"
labels: ["m5"]
order: 2
---

# history, show and delete commands

Auto-save with `--no-save`, list newest first, render stored runs, delete and prune with confirmation.

## Requirements

- [ ] FR-HIST-02 [Must]: `sitevet history [url]` lists past runs with ID, date, URL, page count and overall score, newest first.
  Acceptance: output lists the fixture runs in order.
- [ ] FR-HIST-03 [Must]: `sitevet show <id> [--format]` renders a stored run without crawling again.
  Acceptance: the output equals the report rendered at audit time.
- [ ] FR-HIST-05 [Must]: `sitevet history delete <id>` and `sitevet history prune --keep <n>` remove runs, asking for confirmation unless `--yes` is passed.
  Acceptance: prune with `--keep 2` leaves the newest 2 runs.

Spec: `docs/spec/06-build-spec.md`, section 8.
