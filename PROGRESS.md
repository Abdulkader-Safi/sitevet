# Progress

Build log for sitevet, kept per section 13 of `docs/spec/06-build-spec.md`.

## Milestone status

| Milestone      | Status                                   |
| -------------- | ---------------------------------------- |
| M0 de-risk     | Done. Safi gave the go on 2026-11-10       |
| M1 engine core | Done. CI green on 2026-11-10 (run 42813274176). README in review |
| M2 to M14      | Not started                              |

## M1: engine core

Workspace `crates/sitevet` and `crates/sitevet-cli`, MSRV 1.88. CI (`.github/workflows/ci.yml`): fmt, clippy with `-D warnings`, tests on Linux, macOS and Windows, a check on Rust 1.88, cargo deny.

CI run 42813274176 passed every job: fmt, clippy, tests on all three OSes, the 1.88 check and cargo deny.

Tests: 11 unit tests, 19 integration tests (`tests/crawl.rs`, `tests/engine.rs`, `tests/spawn.rs`) and 2 doc tests, all offline against local wiremock fixture servers.

### Acceptance boxes

`06-build-spec.md` is never edited by hand, so the ticks live here and on the cards.

- [x] FR-API-01: 3 linked pages, score 0 to 100, findings and metrics per page. Uses test rules and a test collector; the built-in rules arrive in M2.
- [x] FR-API-02: event order, then the streamed report equals `audit`'s (timings aside).
- [x] FR-API-03: cancel, stream drop and `audit` future drop on a slow 100-page site leave 0 open connections within 1 s. The "no Chromium process" half is checked again in M4.
- [ ] FR-API-04: the TOML half passes. The flags half needs the CLI (M3).
- [x] FR-API-06: `spawn` from `futures::executor` with no tokio context gets every event and the report, and cancel and drop stop it within 1 s.
- [x] FR-CRAWL-01, FR-CRAWL-02, FR-CRAWL-03, FR-CRAWL-04, FR-CRAWL-05, FR-CRAWL-06, FR-CRAWL-07, FR-CRAWL-08, FR-CRAWL-09, FR-CRAWL-11.
- [x] FR-EVENT-01, FR-EVENT-02.

### Decisions made in M1

None of these swaps a stack choice.

- Report and PageData fields arrive with the milestone that fills them. M1 has no `site`, `skipped_rules`, `changes`, `language` or `screenshots` yet, and PageData holds the DOM reads the crawl and the M2 rules need (title, meta, canonical, headings, links, images, scripts, stylesheets, text). The `Error` enum gets its browser and history variants in M4 and M5. All public types are `#[non_exhaustive]`, so adding them later is not breaking.
- Pages listed only in the sitemap are queued right after the start page at depth 1, so `PageResult.depth` for them is 1. The click depth table in M8 comes from the link graph, not from this number.
- A response that is not HTML (by `Content-Type`; a missing one counts as HTML) is not audited as a page. Its status still goes into the link checker's cache. A non-HTML start page is `Error::NotHtml`.
- Sitemaps: 50,000 URLs in total across all files, not only per file, so a large sitemap index cannot pull in millions of URLs.
- The robots.txt rule shown for a blocked URL is a label: the longest `Disallow` pattern that matches, without checking which agent group it sits in. The decision itself comes from texting_robots.
- A page that fails to fetch (not the start page) still runs every rule, on empty content, so M2 rules can report its status. A rule that panics gives an `info` finding "rule crashed: ..." (the spec's wording; the TS package used `warn`).
- `score_findings` and `compute_overall` are ported now because `PageComplete` needs a page score. The ported TS scoring tests come with M2 (FR-SCORE-01).
- `allow_private_network` is `Option<bool>`: unset means "allowed when the start URL is private", the spec's dynamic default.
- The `browser`, `history` and `media` features are declared (empty) so `Options::validate` can say which feature a setting needs.
- `rustfmt.toml` sets 120 columns. Clippy allows `field_reassign_with_default`, since `#[non_exhaustive]` options can only be built by setting fields after `Options::default()`.

### Open items for Safi

- Character sets. Page bodies are decoded as UTF-8, with invalid bytes replaced. A page in Windows-1252 or Shift_JIS will have garbled text, which matters for the text engine in M6. Decoding other charsets needs a crate such as `encoding_rs` (reqwest's `charset` feature uses it), and that is a stack addition. Add it now, in M6, or not at all?

## M0: de-risk

Spike code lived in `spikes/` (its own cargo package) and ran on three OSes through `.github/workflows/m0-spikes.yml`. M1 removed both; they are in git history up to commit `e920df2`. Local numbers come from an M-series Mac (macOS, arm64). CI numbers come from GitHub's `ubuntu-latest`, `macos-latest` and `windows-latest` runners, over 3 pushes and 1 manual run on 2026-11-10.

Verdict: go. Every exit criterion is met, so stage 4 needs no change.

### 1. Browser spike (chromiumoxide 0.9.1, installed Chrome)

Chrome 157.0.8103.40 locally, 156.0.8078.x on CI. Test page served by a local HTTP server. axe-core 4.14.0 injected unmodified.

| Check                         | macOS (local) | macOS (CI)        | Linux (CI)  | Linux musl binary (CI) | Windows (CI)   |
| ----------------------------- | ------------- | ----------------- | ----------- | ---------------------- | -------------- |
| Launch (cold / warm)          | 5.9 s / 0.4 s | 3.7 to 7.3 s / 1.1 to 1.7 s | 0.6 to 9.2 s / 0.2 s | 0.8 to 7.2 s | 4.5 to 5.0 s / 0.4 s |
| axe violations found          | color-contrast, image-alt, label (all runs, all OSes) | same | same | same | same |
| axe run time                  | 134 to 149 ms | 152 to 997 ms     | 98 to 279 ms | 149 to 209 ms         | 130 to 183 ms  |
| LCP read                      | 36 to 108 ms  | 368 to 3,452 ms   | 52 to 704 ms | 104 to 252 ms         | 804 to 1,628 ms |
| CLS read                      | 0.0433        | 0.0433 (1 miss in 4) | 0.0433   | 0.0433                 | 0.0425         |
| Viewport screenshot           | 1280x657      | 1280x657          | 1280x657    | 1280x657               | 1264x649       |
| Full-page screenshot          | 1280x3715     | 1280x3715         | 1280x3715   | 1280x3715              | 1264x3715      |
| Cancel to zero Chrome processes, hard kill | 35 to 41 ms | 110 to 502 ms | 33 to 171 ms | 37 to 288 ms | 494 to 515 ms |
| Cancel, polite close first (comparison) | 105 to 135 ms | 402 to 1,211 ms | 85 to 261 ms | not run | 731 to 827 ms |
| Server connections open after cancel | 0 | 0 | 0 | 0 | 0 |

Chrome processes are counted by a unique `--user-data-dir` in the command line (`pgrep -f` on Unix, `Get-CimInstance Win32_Process` on Windows), so renderer and GPU children count too. That is 8 to 12 processes per OS.

musl: the spike builds for `x86_64-unknown-linux-musl` with `musl-tools` installed (reqwest with rustls and aws-lc-rs included). The result is a `static-pie` ELF, and that binary runs the full browser spike on Ubuntu with every check passing.

chromey was not needed.

### 2. spawn spike

An owned 2-thread tokio runtime runs a reqwest fetch loop (5 at a time) and sends events over a bounded async-channel. `main` has no tokio context: it drives the handle with `futures::executor::block_on`. `Drop` cancels the token and calls `shutdown_background()`.

| Case                                | macOS (local) | macOS (CI)     | Linux (CI) | Windows (CI) |
| ----------------------------------- | ------------- | -------------- | ---------- | ------------ |
| Full run, 100 pages: events and report | 102 events, report with 100 pages, 506 ms | same, 1.3 to 2.1 s | same, 417 ms | same, 439 to 449 ms |
| Cancel mid-crawl, report returned   | under 1 ms    | under 1 ms     | under 1 ms | under 1 ms   |
| Open connections 300 ms after cancel | 0            | 0              | 0          | 0            |
| Drop handle mid-crawl               | under 1 ms, 0 connections left, no panic | same | same | same |
| Panics seen by the panic hook       | 0             | 0              | 0          | 0            |

### 3. Memory spike

1,000 generated pages parsed with scraper 0.29 inside `spawn_blocking`, 5 at a time. Each page becomes a PageData-like struct (title, meta tags, headings, links, images, script URLs, JSON-LD, full body text, word count) and the DOM is dropped inside the blocking task. All 1,000 structs stay in memory to the end, as the site stage will need them.

Page size mix: 90% about 100 KB, 9% about 500 KB, 1% about 3 MB, plus one 10 MB page (the NFR-05 cap). 172 MB of HTML in total.

| Measure                    | macOS (local) | macOS (CI)    | Linux (CI)    |
| -------------------------- | ------------- | ------------- | ------------- |
| Peak RSS, 1,000 pages      | 399 to 402 MB | 393 to 434 MB | 306 to 330 MB |
| Parse time                 | 0.6 s wall, 2.1 s CPU (about 2 ms CPU per page) | | |
| PageData kept              | 177 MB (181 KB per page) | same | same |

Projected 1,000-page peak: under 500 MB on every run (target NFR-02). Windows was not measured: the spike reads peak RSS through `getrusage`, which Windows lacks.

### 4. MSRV

`cargo msrv find --min 1.85` on the spike's dependency set: 1.88.0. The floor comes from `icu_collections`, `icu_locale_core` and `icu_normalizer` 2.4, which `url` pulls in through `idna`. M1 sets `rust-version = "1.88"` in the workspace.

## Decisions made in M0

These are implementation details inside the decided stack. None of them swaps a stack choice.

- Browser cancel is a hard kill of the Chrome parent process, not `Browser.close`. The profile is throwaway (fresh per audit), so a polite close buys nothing, and on the slow macOS runner it once took 1,211 ms, over the 1 s limit. Killing the parent took every child process with it on all three OSes.
- Open each tab as `about:blank`, call `bring_to_front()`, then `goto(url)`. `browser.new_page(url)` loads the page in a background tab, and Chrome then records no layout shifts during load (CLS read 0 every time until this change).
- Read LCP and CLS by waiting for `PerformanceObserver` entries with a 5 s cap, not with fixed sleeps. Fixed sleeps missed values on the slow macOS runner.
- chromiumoxide 0.9 does not export `HeadlessMode`. Use `BrowserConfig::builder().new_headless_mode()`.
- `spikes/` stays until M1 moves the `spawn` handle code into `crates/sitevet` (the M1 task says "moved over from the M0 spike"). M1 deletes `spikes/` and `m0-spikes.yml`.

## Open items for Safi

- 10,000 pages. NFR-02 caps memory for 1,000 pages only. Keeping full body text in PageData for 10,000 pages (the NFR-05 maximum) projects to about 1.9 to 2.0 GB. The generated pages are text heavy (about 29,000 words each), so real sites should come in lower. If 10,000 pages should also fit a memory budget, stage 3 needs a number for it. Do you want one?
- CLS on the macOS CI runner read 0 in 1 of 4 runs after the wait fix, even with Chrome's anti-throttling flags (chromiumoxide sets them by default). It never happened locally or on Linux and Windows. The spec runs browser tests on Linux CI, so this does not block. M4 should keep an eye on it.
