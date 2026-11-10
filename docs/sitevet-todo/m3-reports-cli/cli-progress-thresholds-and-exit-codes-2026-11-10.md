---
id: "cli-progress-thresholds-and-exit-codes-2026-11-10"
status: "review"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:16:46.000Z"
labels: ["m3", "needs-review"]
order: 6
---

# CLI progress, thresholds and exit codes

Per-page lines, summary block, `--fail-under`, `--fail-on`, exit 0, 1 or 2, `NO_COLOR`, `--quiet`, `--plain`. Snapshot tests.

## Requirements

- [x] FR-CLI-03 [Must]: Default progress shows one line per page (`3/100  https://site/about  92/100`) and a summary with the overall score and the counts of failures per category.
  Acceptance: snapshot test of stderr on a fixture site.
- [x] FR-CLI-04 [Must]: `--fail-under <score>` and `--fail-on error|warning` for CI. Exit codes: 0 when the audit ran and passed thresholds, 1 when a threshold failed, 2 when the audit could not run (bad URL, unreachable start page, bad config, missing browser).
  Acceptance: each exit code is produced by a matching test case.
- [x] FR-CLI-05 [Must]: Output respects `NO_COLOR`, prints status words (PASS, FAIL, WARN) next to colours, and has `--quiet` and `--plain` modes (no animation, no colour).
  Acceptance: with `NO_COLOR=1`, stderr contains no ANSI escape codes.

> [!note]
> Needs a visual check by Safi. Move to `review` when done, not `done`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

stderr: a start line, "crawling...", "crawled N pages, running M checks", one line per page (`3/15  https://site/about  92/100  PASS`), then the overall score and one line per category with fail and warn counts, pages audited, blocked URLs and the report path. A single page with no internal links says "1 page audited, no internal links found". Status words: 90 and up PASS, 50 and up WARN, below FAIL.

Exit 0 passed, 1 when `--fail-under` or `--fail-on error|warning` trips (with a "threshold failed:" line), 2 for errors, each printed as `error: <cause>` with a hint. Colour comes from anstream, so `NO_COLOR`, `CLICOLOR` and non-terminals switch it off; `--plain` forces it off; `--quiet` prints only errors.

Tests in `crates/sitevet-cli/tests/cli.rs`: every exit code, `NO_COLOR` with colour forced on, `--plain`, `--quiet`, and an insta snapshot of stderr on the basic site.

Needs Safi's look at the terminal output.
