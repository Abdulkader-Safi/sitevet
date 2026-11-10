# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project state

This repo is `sitevet`: a Rust rewrite of a website audit engine, shipped as a library crate plus a CLI binary built on that crate. It is in the spec phase. No Rust code exists yet.

- The spec lives in `docs/spec/`, written one stage at a time with the `project-spec` skill: `01-product.md` through `06-build-spec.md`. Only files marked `Status: signed off` are decided. Treat drafts as open.
- Once `06-build-spec.md` is signed off, build from it alone, milestone by milestone, starting with M0. Follow its section 13 (keep `PROGRESS.md` at the repo root, commit per milestone, never add features outside the spec).
- Never edit `06-build-spec.md` by hand to add something. Change the earlier stage it belongs to, then regenerate 06.
- When the spec and reality disagree (a crate doesn't work, a decision is wrong in practice), stop and say so. Don't silently swap a stack choice.

## Reference project (read only)

The TypeScript version lives at `../Safi-Studio-Scanner` (npm `safi-studio-scanner`, v1.0.2). It is the feature baseline: 94 rules in 15 categories, the crawl, findings and metrics, streaming events, the Playwright/axe and PageSpeed providers, and the 10-phase backlog in its `features.md`.

Never modify anything in that repo. Safi is still actively working on it. Read it to port behaviour, rule logic and test fixtures (`test/fixtures/*.html`).

## Architecture (from the spec, carry into code)

- Library first. The CLI is a thin wrapper over the public crate API. Every CLI flag and every key in the optional `sitevet.toml` maps one-to-one to the library options struct, and flags override the file.
- Pipeline: crawl (same-origin pages, plus `robots.txt` and `sitemap.xml` once per site) -> per-page context with the parsed DOM -> run rules and collectors -> score -> render (JSON, Markdown, self-contained HTML).
- Two output channels per page. Findings are verdicts (pass, fail, warn, info, with severity and evidence) and they move the score. Metrics are numbers and tables with their own `ok | warn | bad | missing` status, and they never touch the score. Keep these separate.
- Adding a rule means one file plus one registry line. Keep rule IDs identical to the TS package (`core-seo/title-present`) so docs and habits carry over.
- A shared link checker, so a URL referenced by many rules is fetched once.
- Rendered checks (axe-core, Core Web Vitals, mobile extras) run in local headless Chromium, with PageSpeed Insights as the no-browser fallback. Both stay opt-in.

## Commands

No `Cargo.toml` exists yet. Once it does, the expected commands are standard cargo. Update this section to match the workspace layout the build spec defines.

```bash
cargo build
cargo test                       # all tests
cargo test <test_name>           # one test by name filter
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run -- audit https://example.com --format html --out report.html
```

## Task board (md-kanban)

Track all work as markdown task cards in `docs/sitevet-todo/`, following the `md-kanban` skill. Load that skill before creating or moving cards.

- New tasks always start with `status: "todo"`.
- When you start work on a task, set it to `in-progress`.
- When you finish, set it to `done` and note in the body what was completed.
- If the result needs a visual check by Safi (the HTML report, CLI terminal output, docs pages), set it to `review` instead of `done`.
- Group cards in subfolders by milestone or area, as the build spec defines them. Refresh `modified` on every edit.

## Git

Commit after each step: each signed-off spec stage, each task card moved to `done` or `review`, each finished milestone. One step per commit, with a message naming the step (for example `spec: sign off stage 01 product`). Remote is `origin` at `github.com/Abdulkader-Safi/sitevet`, default branch `main`.

## Writing

Keep `README.md` current as features land: it says what works today, and marks anything planned as planned. The GPUI desktop app is a separate, standalone repo: don't mention it in this repo's README or code.

Use the `ai-writing-rules:anti-ai-writing` skill for every doc, the README, spec files and user-facing strings. Short version: no em dashes, straight quotes, sentence case headings, no filler vocabulary, specific over generic.
