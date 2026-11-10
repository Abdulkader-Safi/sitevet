# Build spec: sitevet

> **This is the hand-off document for Claude Code.** It is self-contained: build from this file alone. It pulls together stages 1 to 5 (product, users, requirements, tech decisions, tech requirements). Where it says "see Stage N", that is optional background.

Status: signed off 2026-11-10. Sources: 01 to 05 as of that date.

## 1. What we're building

sitevet is a website audit engine in Rust, shipped as a library crate (`sitevet`) and a CLI (`sitevet-cli`, binary `sitevet`) built on that library. It crawls a site, runs 94 rules ported from an earlier TypeScript package plus roadmap phases 2 to 9 (text and readability, answer engine, site-wide duplicates and orphans, trust signals with local SEO, performance depth, tech stack detection, media, then mobile and rendered extras), and returns a scored report as JSON, Markdown or one self-contained HTML file. It is for two equal primary users: Rust developers who embed audits in their own apps (the first one is a GPUI desktop app in a separate repo), and freelancers or CI pipelines running quick audits from the terminal. The difference from existing tools: the engine is an open, MIT-licensed library with typed reports, streaming progress events and pluggable rules, where competitors lock the engine inside a desktop app, a SaaS, or a binary.

## 2. Stack (decided, see Stage 4)

| Layer                                    | Choice                                                                                                                                |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Language                                 | Rust edition 2024, MSRV pinned in M0, no newer than 1.90                                                                              |
| Layout                                   | Cargo workspace: `crates/sitevet` (library), `crates/sitevet-cli` (binary `sitevet`)                                                  |
| Async runtime                            | tokio 1.54, plus an owned 2-thread runtime for `spawn()`. Events on async-channel 2.5. Cancel with tokio-util 0.7 `CancellationToken` |
| HTTP                                     | reqwest 0.13 with rustls, gzip, brotli, zstd, http2, manual redirects                                                                 |
| HTML parsing                             | scraper 0.29 (html5ever 0.40)                                                                                                         |
| Crawl inputs                             | url 2.5, texting_robots 0.2 (fallback robotstxt), quick-xml 0.42                                                                      |
| Browser                                  | chromiumoxide 0.9 behind the `browser` feature, user's installed Chrome (fallback crate: chromey)                                     |
| Accessibility                            | axe-core 4.14.0, bundled unmodified (MPL-2.0)                                                                                         |
| PageSpeed                                | PSI v5 REST through reqwest                                                                                                           |
| Text engine                              | unicode-segmentation 1.13, whatlang 0.18, stop-words 0.10, rust-stemmers 1.2, readability in-crate (English formulas)                 |
| Structured data                          | serde_json for JSON-LD, in-crate microdata and RDFa                                                                                   |
| Site-wide                                | in-crate SimHash over 5-shingles, hash-map duplicate groups, BFS link graph                                                           |
| Fingerprinting                           | own `data/fingerprints.toml`, regex 1.13, aho-corasick 1.1                                                                            |
| Media                                    | imagesize 0.15 over ranged GET                                                                                                        |
| Report rendering                         | askama 0.16 (HTML, Markdown), serde JSON, schemars 1.2 for the JSON Schema                                                            |
| History                                  | rusqlite 0.41 (bundled) behind the `history` feature, zstd blobs, directories 6                                                       |
| CLI                                      | clap 4.6 derive, toml 1.2, anstream and anstyle, anyhow                                                                               |
| Errors and logs                          | thiserror 2.0 (library), tracing 0.1                                                                                                  |
| Testing                                  | cargo test, wiremock 0.6 fixture server, insta 1.49, jsonschema 0.61, assert_cmd                                                      |
| CI                                       | GitHub Actions: fmt, clippy, tests on three OSes, browser tests on Linux, cargo deny                                                  |
| Distribution                             | crates.io (both crates), cargo-dist 0.34: GitHub Releases for 5 targets, installers for shell and PowerShell, Homebrew tap            |
| Docs                                     | README, docs.rs, CONTRIBUTING.md, generated `docs/rules.md`                                                                           |
| Hosting, database server, auth, payments | none: local library and CLI                                                                                                           |

## 3. Architecture and principles

```
            sitevet-cli (binary "sitevet")
   clap flags + sitevet.toml -> Options      progress lines, exit codes
   history / show / diff / rules / init       (stderr)    report (stdout or --out)
                     |                                  ^
                     v                                  |
 +----------------------------- sitevet (library) ------------------------------+
 | entry: audit() / audit_stream() / spawn() -> AuditHandle                      |
 |                                                                               |
 |  Crawler ----> Fetcher (reqwest, per-host semaphore + delay, redirects)       |
 |    |  robots.txt + sitemap (once)        LinkChecker (dedup cache, once/URL)  |
 |    v                                                                          |
 |  per page:  [blocking]  parse HTML (scraper) -> DOM rules + collectors        |
 |                          -> extract PageData (Send), drop DOM                 |
 |             [async]     enrich: link status, media probes                     |
 |                          -> enriched rules + collectors                       |
 |             [async]     render (browser or PSI) -> rendered rules + collectors|
 |    v                                                                          |
 |  after crawl: site rules + site collectors over all PageData                  |
 |    v                                                                          |
 |  Scorer -> AuditReport ---> render(json | md | html)   History (SQLite)       |
 |                                                                               |
 |  events: AuditEvent --> async-channel / Stream --> caller                     |
 +-------------------------------------------------------------------------------+
        | HTTP(S)                 | CDP                        | HTTPS
        v                         v                            v
   audited site + links     local Chrome/Chromium        PageSpeed Insights
```

**Non-negotiable principles:**

1. Library first : all audit logic is in `crates/sitevet`. The CLI only parses flags and TOML into `Options`, calls the library, then prints output and sets the exit code. Every `Options` field is reachable from both a flag and a `sitevet.toml` key.
2. PageData is the keystone : the DOM (`scraper::Html`, not `Send`) lives inside one `spawn_blocking` call per page. Everything after that reads the extracted `PageData`. Never hold `Html` across an `.await`.
3. Findings and metrics stay separate : findings are verdicts and move the score. Metrics are values and tables with their own status and never touch the score.
4. Parity by source : rule IDs, severities, rule conditions, category names and the scoring formula match the TS package at commit `de27aea` (`../Safi-Studio-Scanner`, read only). Never modify that repo.
5. Cancellation is total : one `CancellationToken` per audit, checked before every request and around every await. Cancel or drop returns within 1 second with no open connections and no Chrome process.
6. Heavy parts are opt-in : `browser`, `history` and `media` code sit behind cargo features that are off by default in the library. The CLI enables `history` and `browser`.
7. Everything the site sends is hostile : cap every size, catch rule panics, escape all page text in reports, block private addresses when the start URL is public, keep Chrome's sandbox on, never log or store `psi_key`.
8. No network beyond the audit : no telemetry, no update checks. Only the audited site, the links it checks, and PSI when a key is set.
9. No company branding : nothing in the repo (code, reports, docs) mentions the author's former company brand. Neutral sitevet styling.

## 4. Repository structure

```
Cargo.toml                    workspace, shared lints, version pins
deny.toml                     cargo deny licence and advisory rules
README.md  CONTRIBUTING.md  THIRD_PARTY.md  LICENSE  PROGRESS.md
schema/report.schema.json     generated JSON Schema, committed, checked in tests
docs/
  spec/                       this spec (01 to 06)
  sitevet-todo/               md-kanban task cards, one folder per milestone
  rules.md                    generated by `sitevet rules --format md`
crates/
  sitevet/
    src/
      lib.rs                  public API re-exports
      options.rs              Options, validation, TOML parsing
      error.rs                sitevet::Error
      engine/                 audit(), audit_stream(), spawn(), cancellation, events
      crawl/                  fetcher, queue, robots, sitemap, link checker, ssrf guard
      extract/                DOM -> PageData (links, images, text, schema, fingerprints)
      rules/<category>.rs     one file per category, each rule a unit struct
      rules/mod.rs            registry: all_rules(), select_rules()
      metrics/<category>.rs   collectors, same pattern
      site/                   site rules and collectors (phase 4)
      text/                   tokenizer, stop words, stems, n-grams, readability
      render_stage/           browser (feature "browser") and psi providers
      media/                  feature "media"
      score.rs                score_findings(), compute_overall()
      report/                 types, json, markdown and html (askama templates), diff
      history/                feature "history": SQLite store and migrations
    data/                     fingerprints.toml, authoritative-domains.txt
    assets/                   axe.min.js + licence files, report.css
    templates/                askama templates
    tests/
      support/                fixture server
      fixtures/pages/         single-page HTML fixtures (ported from the TS package)
      fixtures/sites/<name>/  multi-page fixture sites with site.toml
  sitevet-cli/
    src/main.rs, args.rs, config.rs, output.rs, commands/
    tests/                    assert_cmd tests and insta snapshots
examples/
  spawn-futures.rs            spawn() from a non-tokio executor
  custom-rule.rs              adding a rule through the Rule trait
  serve-fixture.rs            serves a fixture site for demos
```

## 5. Data model

Full tables in stage 5. Conventions: snake_case JSON, RFC 3339 UTC times, integer `_ms` and `_bytes`, rule and metric IDs as `category/slug`, `#[non_exhaustive]` on public types.

- Options : every audit setting with defaults (100 pages, depth 5, concurrency 5 per host, delay 0, timeout 15 s, robots obeyed, sitemap crawl on, browser off, PSI cap 5, media off with a 200-request cap, screenshots off). CLI-only keys sit in a `[cli]` table in `sitevet.toml`.
- AuditReport : `schema_version` (1), versions, URLs, time, duration, options summary without secrets, overall score, category scores, pages, site section, blocked URLs, skipped rules, optional changes.
- CategoryScore : category, score, and a count per status.
- PageResult : URL, final URL, status, error, depth, redirect chain, timings, size, truncated flag, language info, score, findings, metrics, screenshots.
- Finding : rule ID, category, title, status, severity, message, details, evidence (2,000 char cap), fix, stable `key` for diffs.
- Metric : ID, category, label, optional status, note, and a value (with unit) or a table (columns, rows capped at 25, total).
- SiteResult : site findings (each with affected URLs) and site metrics.
- PageData (internal) : everything extracted from one page's DOM, plus rendered data when that stage runs.
- AuditEvent : AuditStart, PageFetched, CrawlComplete, PageRendered, RenderComplete, CheckComplete, PageComplete, AuditComplete (with the report). `Send + Clone + Serialize`.
- History run : SQLite `runs` table with integer ID, URL, origin, start time, duration, pages, score, versions, options JSON, zstd report blob. `PRAGMA user_version` for migrations.
- ReportDiff : baseline reference, score change (overall, per category), new failures, fixed findings, pages added and removed.
- Bundled data : fingerprint signatures, authoritative domain list, axe-core, report CSS.

## 6. API / command surface

Library:

- `audit(url, Options) -> Result<AuditReport>`
- `audit_stream(url, Options) -> AuditStream` (a `Stream` of events, plus `final_report()`)
- `spawn(url, Options) -> AuditHandle` (`events()`, `cancel()`, `report()`, works on any executor)
- `audit_to_html`, `audit_to_markdown`, `audit_score`
- `Options::default()`, `Options::from_toml_str()`, `Options::validate()`
- `render(&AuditReport, Format)`, `render_with(&AuditReport, Format, RenderOptions)`
- `diff(&AuditReport, &AuditReport) -> ReportDiff`
- `all_rules()`, `all_collectors()`, `select_rules(&Options)`, `report_schema()`
- Traits: `Rule`, `SiteRule`, `Collector`, `SiteCollector`, with `Stage::{Dom, Enriched, Rendered}` and `RuleCx` / `SiteCx`
- History (feature `history`): `History::open`, `save`, `list`, `load`, `delete`, `prune`, `latest_for`
- Errors: `InvalidUrl`, `InvalidOptions`, `StartPageUnreachable`, `NotHtml`, `BrowserNotFound`, `BrowserFailed`, `History`, `Cancelled`

CLI (exit 0 ran and passed, 1 threshold failed, 2 could not run):

- `sitevet audit <url>` with one flag per option, plus `--format`, `--out`, `--config`, `--fail-under`, `--fail-on`, `--baseline <id|last|file>`, `--no-save`, `--quiet`, `--plain`, `--verbose`
- `sitevet history [url] [--limit]`, `sitevet history delete <id> [--yes]`, `sitevet history prune --keep <n> [--yes]`
- `sitevet show <id> [--format] [--out]`
- `sitevet diff <id-a> <id-b> [--format]`
- `sitevet rules [--category] [--format table|md|json]`
- `sitevet init [--force]`
- `sitevet --version`, `sitevet help`

## 7. Screens

- CLI audit run : start line, one line per finished page (`3/100  https://site/about  92/100` with a status word), summary with overall and per-category scores, the number of blocked URLs and skipped rules, the report path and the run ID. States: crawling, error (`error:` line plus hint, exit 2), single page with no links, success.
- CLI history, show, diff, rules : aligned plain tables. Empty state "No runs saved yet. Run sitevet audit <url>." Unknown run ID is exit 2.
- HTML report : header with the URL and date next to the score badge. Changes panel with a baseline. Category grid. Site-wide section. Per-page `<details>` sections with findings by category and metric tables. Screenshot thumbnails embedded as base64, full-page files linked. Footer with versions. One file, no JavaScript needed, no external requests, WCAG 2.2 AA, readable at 360 px, light and dark themes, `dir="auto"` on quoted page text.
- Markdown report : summary and category tables, site-wide section, per-page findings and metric tables, escaped page text.

## 8. Build plan (milestones)

Each milestone gets a folder of task cards in `docs/sitevet-todo/` before work starts. Releases are 0.x crates so a stall still leaves something usable.

### M0: de-risk (do this first)

A throwaway spike in `spikes/` (deleted or moved into the real crates once proven). Proves the top 3 risks from stage 4.

- **Tasks:**
  - Browser spike on macOS, Linux and Windows: launch the installed Chrome with chromiumoxide 0.9, load a page, inject axe-core 4.14 and read violations, read LCP and CLS through `PerformanceObserver`, take a full-page and a viewport screenshot, cancel mid-load and confirm no Chrome process remains after 1 s. Check chromiumoxide builds for `x86_64-unknown-linux-musl`.
  - `spawn` spike: an owned 2-thread tokio runtime, a reqwest fetch loop sending events over async-channel, driven from `futures::executor::block_on` with no tokio context. Cancel, drop the handle, and exit the process with no panic (`shutdown_background()` in `Drop`).
  - Memory spike: parse 1,000 generated fixture pages through `spawn_blocking`, extract a `PageData`-like struct, drop the DOM, and measure peak resident memory.
  - Run `cargo msrv find` on the spike's dependency set and record the MSRV.
- **Exit criteria:** all four results written into `PROGRESS.md` with numbers. Browser works on all three OSes, or chromey is tried and works. `spawn` cancels within 1 s with no panic. Projected memory for 1,000 pages is under 500 MB. MSRV is 1.90 or older. Fallback if the browser fails on one OS: stop and report before M1, since stage 4 must change.

### M1: engine core

- **Builds:** FR-API-01, FR-API-02, FR-API-03, FR-API-04, FR-API-06, FR-CRAWL-01, FR-CRAWL-02, FR-CRAWL-03, FR-CRAWL-04, FR-CRAWL-05, FR-CRAWL-06, FR-CRAWL-07, FR-CRAWL-08, FR-CRAWL-09, FR-CRAWL-11, FR-EVENT-01, FR-EVENT-02
- **Tasks:**
  - Workspace with both crates, shared lints, CI (fmt, clippy, tests on three OSes, cargo deny).
  - `Options` with defaults and validation, plus TOML parsing.
  - Fixture server in `tests/support` and the first fixture sites.
  - Fetcher with per-host semaphore, delay, timeouts, manual redirects, size caps and the SSRF guard.
  - Crawler: BFS queue, URL normalisation, robots.txt obeying, sitemap seeding, blocked list.
  - Shared link checker with a per-URL shared future.
  - PageData extraction in `spawn_blocking` (no rules yet), events, cancellation token.
  - `audit`, `audit_stream` and `spawn` (moved over from the M0 spike), with the `spawn-futures` example.
- **Exit criteria:** a fixture site crawls with correct limits and events from all three entry points, cancel returns in under 1 s, CI is green on three OSes.

### M2: parity rules, metrics, scoring

- **Builds:** FR-RULE-01, FR-RULE-02, FR-RULE-03, FR-RULE-04, FR-MET-01, FR-MET-02, FR-SCORE-01, FR-SCORE-02, FR-EXT-01, FR-EXT-02
- **Tasks:**
  - `Rule`, `SiteRule`, `Collector`, `SiteCollector` traits and `RuleCx`, with stage scheduling and panic catching.
  - Port the TS fixtures from commit `de27aea` into `tests/fixtures/pages/`.
  - Port the 94 static rules category by category (one task card per category), each with pass and fail fixture tests. The browser-stage rules get their logic now and run in M4.
  - Port every collector from `src/metrics/*.ts` with the 25-row table cap.
  - Port `score_findings` and `compute_overall` with the TS test cases.
  - `CONTRIBUTING.md` with the one-file rule example, and the `custom-rule` example.
  - Compare against the TS repo's current HEAD and log differences in `PROGRESS.md`.
- **Exit criteria:** every static rule and collector passes its fixtures, scores match the TS package for the same findings.

### M3: reports and CLI (release 0.1.0)

- **Builds:** FR-API-05, FR-REPORT-01, FR-REPORT-02, FR-REPORT-03, FR-REPORT-04, FR-CLI-01, FR-CLI-02, FR-CLI-03, FR-CLI-04, FR-CLI-05, FR-CLI-06, FR-DIST-01, FR-DIST-02
- **Tasks:**
  - JSON output, the generated JSON Schema, and the schema test.
  - Markdown and HTML templates with askama, neutral styling with light and dark themes, `dir="auto"` on page text. HTML goes to `review` for Safi.
  - CLI: `audit` with all flags, `sitevet.toml` with unknown-key suggestions, progress lines, thresholds with exit codes, `NO_COLOR`, `--quiet` and `--plain`, `rules`.
  - `sitevet init` (Should).
  - cargo-dist setup, publish 0.1.0 of both crates, GitHub Release with binaries and installers.
- **Exit criteria:** `cargo install sitevet-cli` and the shell installer work on a clean machine, a fixture audit renders in all three formats, CLI snapshots pass. Safi signs off the HTML report look.

### M4: rendered stage

- **Builds:** FR-RENDER-01, FR-RENDER-02, FR-RENDER-03, FR-RENDER-04, FR-RENDER-05, FR-RENDER-06
- **Tasks:**
  - `browser` feature: Chrome detection, guarded browser process, fresh profile per audit, tab pool of 2.
  - axe-core injection with one finding per violation. Core Web Vitals and page stats.
  - PSI provider with recorded responses, `psi_max_pages`, 429 handling.
  - Skipped-rule reporting and the missing-browser error.
  - Rendered crawl (`render_crawl`, Should).
  - Linux CI job with Chrome for browser tests.
- **Exit criteria:** browser and PSI rules fill in on fixtures, a default build has no browser crates, cancel kills Chrome.

### M5: run history (release 0.2.0)

- **Builds:** FR-HIST-01, FR-HIST-02, FR-HIST-03, FR-HIST-04, FR-HIST-05
- **Tasks:**
  - `history` feature: SQLite store, migrations, zstd blobs, `SITEVET_HOME`.
  - CLI: auto-save with `--no-save`, `history`, `show`, `diff`, `--baseline <id|last|file>`, delete and prune with confirmation.
  - Changes panel in the HTML and Markdown reports.
- **Exit criteria:** two runs on a fixture site show a correct diff, prune keeps the right runs.

### M6: text engine (phase 2)

- **Builds:** FR-TEXT-01, FR-TEXT-02, FR-TEXT-03, FR-TEXT-04, FR-TEXT-05, FR-TEXT-06, FR-TEXT-07, FR-TEXT-08
- **Tasks:**
  - Language detection with confidence, main-content extraction.
  - Unicode tokenizer, stop words and stems per language, n-grams, keyword spans.
  - Text statistics, English readability, the neutral fallback, the no-word-space fallback.
  - Rewrite `content/keyword-stuffing` on the n-gram output.
  - Fixtures with hand-counted values for English, Arabic, a German or French page, and a Chinese page.
- **Exit criteria:** language fixtures match hand-counted values, non-English pages never get an English readability score.

### M7: answer engine and content structure (phase 3)

- **Builds:** FR-AEO-01, FR-AEO-02, FR-AEO-03, FR-AEO-04, FR-AEO-05, FR-AEO-06
- **Tasks:**
  - Answer snippet and Q&A checks. Counts of answer and passage formats.
  - Topic cluster signals with main-content-only links.
  - AI crawler access, `llms.txt`, content present without JavaScript.
  - Microdata and RDFa extraction into the shared schema tree. FAQPage and HowTo validation.
  - Page-type hint (Should).
- **Exit criteria:** every phase 3 fixture passes. Microdata and JSON-LD twins produce the same tree.

### M8: site-wide intelligence (phase 4)

- **Builds:** FR-SITE-01, FR-SITE-02, FR-SITE-03, FR-SITE-04, FR-SITE-05, FR-SITE-06
- **Tasks:**
  - Site rules and collectors run after the crawl in `spawn_blocking`.
  - Duplicate groups, SimHash near-duplicates, orphans, link graph, click depth, redirect map, sitemap reconciliation.
  - Site findings in category scores and the overall penalty, scaled by affected pages.
  - Site-wide section in both report formats.
- **Exit criteria:** the multi-page fixture site reports each known site issue exactly once.

### M9: trust and compliance (phase 5)

- **Builds:** FR-TRUST-01, FR-TRUST-02, FR-TRUST-03, FR-TRUST-05
- **Tasks:**
  - Author box, author link, bio, dates as values.
  - Structured data trust signals table.
  - Checks for refund pages and contact details. Disclosure and editorial policy checks.
  - Local SEO: LocalBusiness completeness, NAP consistency, business profile link.
  - Citation quality with `data/authoritative-domains.txt` (Should).
- **Exit criteria:** phase 5 fixtures pass.

### M10: performance depth (phase 6)

- **Builds:** FR-PERF-01, FR-PERF-02, FR-PERF-03, FR-PERF-04
- **Tasks:**
  - Navigation and paint timings from the rendered stage, INP state, layout shift details, slow resources.
  - Static checks: font loading, HTTP version, resource hints, third-party scripts.
- **Exit criteria:** rendered and static performance fixtures pass.

### M11: tech stack fingerprinting (phase 7)

- **Builds:** FR-TECH-01, FR-TECH-02, FR-TECH-03
- **Tasks:**
  - `data/fingerprints.toml` with the vendors listed in FR-TECH-01 and FR-TECH-02, written from scratch (no Wappalyzer data).
  - Matcher over markup, script URLs, cookies, meta tags and headers. Per-vendor tables replace `analytics/tag-present`'s single result in the metrics while keeping the rule.
- **Exit criteria:** recorded fixtures detect the expected vendors with no false positives across the fixture set.

### M12: media and assets (phase 8)

- **Builds:** FR-MEDIA-01, FR-MEDIA-03, FR-MEDIA-04
- **Tasks:**
  - `media` feature: ranged GET probes with the request cap, per-image table.
  - Responsive image checks, plus video and icon checks.
  - Oversized-image check from the rendered stage.
  - Optimisation estimates (Should).
- **Exit criteria:** with media off no image requests are made, with it on the fixture table is correct.

### M13: mobile and rendered extras (phase 9)

- **Builds:** FR-MOBILE-01, FR-MOBILE-02, FR-MOBILE-03, FR-MOBILE-04
- **Tasks:**
  - Mobile viewport checks: tap targets, font size, overflow, interstitials.
  - Viewport meta checks without a browser.
  - Console errors and the JavaScript-off content comparison.
  - Screenshots with base64 thumbnails in the HTML report. HTML goes to `review` for Safi.
- **Exit criteria:** phase 9 fixtures pass, screenshots render offline in the report.

### M14: hardening and 1.0

- **Builds:** no new requirements; NFR-01 to NFR-11 verification
- **Tasks:**
  - Run the 100-page and 1,000-page benchmark tests and record the numbers against NFR-01 and NFR-02.
  - Check binary size and dependency licences against the NFR checklist below, and confirm the MSRV.
  - README, CONTRIBUTING, generated `docs/rules.md`, crate docs with examples for every entry point.
  - Homebrew tap (Should).
  - Publish 1.0.0 of both crates and the GitHub Release.
- **Exit criteria:** every box in section 9 is ticked, every NFR box is ticked, 1.0.0 is installable from crates.io, the shell installer and Releases.

## 9. Acceptance criteria

One checkbox per Must requirement, generated from stage 3. POC proves the core idea (a typed, streaming, parity audit from library and CLI). MVP is the shippable v1.0.

### POC (M1 to M3)

- [ ] **FR-API-01** Given a local fixture server with 3 linked pages, `audit` returns a report with 3 pages, a score from 0 to 100, findings and metrics per page.
- [ ] **FR-API-02** The stream yields `AuditStart`, then at least one `PageFetched`, then `CrawlComplete`, then `CheckComplete` and `PageComplete` events, then `AuditComplete` with a report equal to what `audit` returns for the same input.
- [ ] **FR-API-03** Cancelling a 100-page audit mid-crawl leaves no open connections and no Chromium process after 1 second.
- [ ] **FR-API-04** A test sets every field through TOML and through flags and gets equal `Options` values.
- [ ] **FR-API-05** Each helper's output equals `render` on the report from `audit` for the same site.
- [ ] **FR-API-06** An example that calls `spawn` from a `futures::executor` (no tokio) receives every event and the final report for a fixture audit, and cancel stops it within 1 second.
- [ ] **FR-CRAWL-01** On a fixture site with 150 linked pages, defaults crawl exactly 100 pages, none deeper than 5 clicks.
- [ ] **FR-CRAWL-02** `max_pages = 10001` fails validation with a message naming the cap.
- [ ] **FR-CRAWL-03** A fixture server logs exactly one request each for `/robots.txt` and `/sitemap.xml` during a 20-page audit.
- [ ] **FR-CRAWL-04** A disallowed path is not requested by default, is listed as blocked, and is requested when `ignore_robots = true`.
- [ ] **FR-CRAWL-05** A page listed only in the sitemap and linked from nowhere is crawled.
- [ ] **FR-CRAWL-06** With `delay_ms = 500`, request start times to one host are at least 500 ms apart.
- [ ] **FR-CRAWL-07** The fixture server sees `sitevet/<version> (+https://github.com/Abdulkader-Safi/sitevet)`.
- [ ] **FR-CRAWL-08** `/a`, `/a/` and `/a#x` on a fixture site produce one audited page.
- [ ] **FR-CRAWL-09** A URL linked from 10 pages is requested once.
- [ ] **FR-CRAWL-11** An unreachable start URL returns an error; one 500 page among 10 produces a report with 10 pages, one marked with its status.
- [ ] **FR-RULE-01** Each rule has a fixture test in which it passes on a good fixture and fails on a bad one. The TS package's fixtures (`test/fixtures/*.html`) are ported and every expected result matches.
- [ ] **FR-RULE-02** The JSON schema test validates every finding from the fixture suite.
- [ ] **FR-RULE-03** A fixture with 3 axe violations yields 3 accessibility findings with distinct IDs.
- [ ] **FR-RULE-04** `only = ["security"]` runs only security rules and collectors, and the report has no other categories.
- [ ] **FR-MET-01** Each collector has a fixture test. A test confirms the score is identical with collectors on and off.
- [ ] **FR-MET-02** A fixture with 300 links gives a link table of at most the cap and a total of 300.
- [ ] **FR-SCORE-01** The ported TS scoring tests produce identical numbers.
- [ ] **FR-SCORE-02** Toggling collectors and skipped rules leaves the score unchanged.
- [ ] **FR-REPORT-01** Every fixture report validates against the schema.
- [ ] **FR-REPORT-02** A snapshot test of the fixture report passes, and the output renders on GitHub.
- [ ] **FR-REPORT-03** The file opens offline with zero network requests and passes an axe check at WCAG 2.2 AA.
- [ ] **FR-REPORT-04** An Arabic fixture's title shows right to left in the HTML report.
- [ ] **FR-EVENT-01** A fixture audit's event log matches the expected sequence and counts.
- [ ] **FR-EVENT-02** A compile test sends events through a `std::sync::mpsc` channel and serialises one to JSON.
- [ ] **FR-CLI-01** `sitevet audit URL --format json > r.json` produces valid JSON, with nothing else in the file.
- [ ] **FR-CLI-02** A misspelled key fails with its name and the closest valid key.
- [ ] **FR-CLI-03** Snapshot test of stderr on a fixture site.
- [ ] **FR-CLI-04** Each exit code is produced by a matching test case.
- [ ] **FR-CLI-05** With `NO_COLOR=1`, stderr contains no ANSI escape codes.
- [ ] **FR-CLI-06** The list has the same count as the registry.
- [ ] **FR-EXT-01** An example crate adds one custom rule and sees its findings in the report.
- [ ] **FR-EXT-02** The contributing guide's worked example compiles and passes.
- [ ] **FR-DIST-01** A clean machine runs `cargo install sitevet-cli` and then `sitevet --version`. A project with `sitevet` as a dependency compiles without clap or rusqlite in `cargo tree`.
- [ ] **FR-DIST-02** The installer works on a fresh macOS and a fresh Ubuntu machine.

### MVP (M4 to M14)

- [ ] **FR-RENDER-01** On a fixture with a missing form label, an `accessibility/*` failure for `label` appears.
- [ ] **FR-RENDER-02** The `performance/lcp` and `performance/cls` rules report numbers for a rendered fixture.
- [ ] **FR-RENDER-03** With a recorded PSI response, the same rules fill in, and only 5 requests are made for a 20-page audit.
- [ ] **FR-RENDER-04** A default audit lists the browser rules as "skipped: needs browser or psi_key", and they do not affect the score.
- [ ] **FR-RENDER-05** With `chrome_path` pointing nowhere, the error names the option and the install hint.
- [ ] **FR-RENDER-06** `cargo tree --no-default-features` shows no browser crates.
- [ ] **FR-TEXT-01** An Arabic fixture with no `lang` attribute is detected as Arabic.
- [ ] **FR-TEXT-02** A fixture whose footer repeats a word 50 times does not show that word in the top keywords.
- [ ] **FR-TEXT-03** English and Arabic fixtures produce correct word counts and stop-word percentages against hand-counted values.
- [ ] **FR-TEXT-04** Fixture values match hand-computed values within 1%.
- [ ] **FR-TEXT-05** The English fixture matches reference values. Arabic and Chinese fixtures show the "not available" state, never an English score.
- [ ] **FR-TEXT-06** A fixture with a known repeated phrase lists it first among 2-word phrases.
- [ ] **FR-TEXT-07** Spans in the fixture point at the exact byte ranges of the keyword.
- [ ] **FR-TEXT-08** A stuffed fixture fails, a normal one passes.
- [ ] **FR-AEO-01** A fixture with a 40-word definition in block 2 reports block 2, 40 words.
- [ ] **FR-AEO-02** A fixture with 4 question H2s, 3 answered, reports 0.75.
- [ ] **FR-AEO-03** Fixture counts match.
- [ ] **FR-AEO-04** Nav links are excluded from the main-content link count on the fixture.
- [ ] **FR-AEO-05** A robots fixture blocking GPTBot reports GPTBot blocked and the others allowed.
- [ ] **FR-AEO-06** A microdata fixture produces the same tree as its JSON-LD twin.
- [ ] **FR-SITE-01** A fixture site with 3 pages sharing one title reports one group of 3.
- [ ] **FR-SITE-02** Two fixture pages differing by one paragraph are flagged. Two unrelated pages are not.
- [ ] **FR-SITE-03** The sitemap-only fixture page is reported as orphaned.
- [ ] **FR-SITE-04** Fixture counts match the known graph.
- [ ] **FR-SITE-05** A link to `/old` that redirects to `/new` appears with both.
- [ ] **FR-SITE-06** The fixture site produces one entry in each list.
- [ ] **FR-TRUST-01** The article fixture reports author name, link, bio present and both dates.
- [ ] **FR-TRUST-02** Fixture table matches.
- [ ] **FR-TRUST-03** An English fixture with a `/refund-policy` link passes the refund check.
- [ ] **FR-TRUST-05** A fixture whose page phone differs from its schema phone reports a NAP mismatch.
- [ ] **FR-PERF-01** All values are present and non-negative for a rendered fixture.
- [ ] **FR-PERF-02** A default rendered audit shows that state.
- [ ] **FR-PERF-03** A fixture with a delayed image lists it.
- [ ] **FR-PERF-04** Each check passes and fails on matching fixtures.
- [ ] **FR-TECH-01** A fixture with GA4 and the Meta pixel lists exactly those two.
- [ ] **FR-TECH-02** Recorded responses from a WordPress-on-Cloudflare fixture detect both.
- [ ] **FR-TECH-03** A test adds a signature in test data and detects it.
- [ ] **FR-MEDIA-01** With the option off, no image requests are made. With it on, the fixture table lists dimensions and sizes.
- [ ] **FR-MEDIA-03** Each check passes and fails on matching fixtures.
- [ ] **FR-MEDIA-04** A 2000 px image shown at 300 px is flagged.
- [ ] **FR-MOBILE-01** Each is flagged on a matching fixture.
- [ ] **FR-MOBILE-02** `user-scalable=no` is flagged.
- [ ] **FR-MOBILE-03** A fixture that throws on load reports the error. An SPA fixture reports the missing content share.
- [ ] **FR-MOBILE-04** With the option on, PNG files exist, the HTML report shows the thumbnails offline, and the full-page links open the files.
- [ ] **FR-HIST-01** Two runs create two rows. A run with `--no-save` creates none.
- [ ] **FR-HIST-02** Output lists the fixture runs in order.
- [ ] **FR-HIST-03** The output equals the report rendered at audit time.
- [ ] **FR-HIST-04** Fixing one fixture rule between runs shows it under fixed, with the category score change.
- [ ] **FR-HIST-05** Prune with `--keep 2` leaves the newest 2 runs.

### Should (build in the listed milestone if time allows)

- [ ] FR-CRAWL-10 On a fixture SPA whose links exist only after JS runs, `render_crawl = true` finds them and the default crawl does not.
- [ ] FR-AEO-07 The article fixture is labelled article.
- [ ] FR-TRUST-04 A fixture citing `who.int` and a random blog reports 50%.
- [ ] FR-MEDIA-02 A large PNG fixture gets an estimate.
- [ ] FR-CLI-07 The written file parses back into default `Options`.
- [ ] FR-DIST-03 `brew install` installs a working binary.

### Non-functional (checked in M14, and NFR-11 at every milestone)

- [ ] **NFR-01** Performance: a 100-page static audit (no browser) of a typical site finishes in under 60 s on a 16 GB M1 over a 100 Mbit connection, with the site's response time as the main cost. Rule and collector CPU time is under 50 ms per page on average.
- [ ] **NFR-02** Memory: a 1,000-page static audit stays under 500 MB resident memory.
- [ ] **NFR-03** Privacy: no telemetry, no update pings, no network requests except to the audited site, the external links it checks, and PSI when a key is set. History stays on the local disk.
- [ ] **NFR-04** Politeness: at most `concurrency` requests in flight per host (default 5), and an optional per-host delay. The default user agent names sitevet and links to the repo.
- [ ] **NFR-05** Scale: up to 10,000 pages per audit, HTML pages up to 10 MB each (larger pages are truncated and flagged), report tables capped.
- [ ] **NFR-06** Platforms: library builds on stable Rust for all three desktop OSes (MSRV set in stage 4). CLI binaries for the five targets in FR-DIST-02.
- [ ] **NFR-07** Accessibility and locale: HTML report at WCAG 2.2 AA, CLI per FR-CLI-05. Tool text in English. Page analysis in any language.
- [ ] **NFR-08** Dependency weight: the browser, history and media code all sit behind cargo features that are off by default. The CLI binary without browser support is under 25 MB.
- [ ] **NFR-09** API stability: the library follows semver from 1.0. The JSON report carries `schema_version`, and any breaking change to it bumps that version.
- [ ] **NFR-10** Licensing: sitevet is MIT. Every dependency, bundled script (axe-core) and data file has a licence compatible with distributing an MIT binary.
- [ ] **NFR-11** Test coverage: every rule and collector has at least one passing and one failing fixture test, run offline against local fixtures.

## 10. Standard states

| Item                               | Coverage                                                             |
| ---------------------------------- | -------------------------------------------------------------------- |
| Undo and redo                      | Out of scope: no editing, runs are read-only snapshots.              |
| Empty states                       | FR-CLI-03, FR-HIST-02, FR-RENDER-04.                                 |
| Loading and progress               | FR-API-02, FR-EVENT-01, FR-CLI-03.                                   |
| Error states and messages          | FR-CRAWL-11, FR-RENDER-05, FR-CLI-02, FR-CLI-04.                     |
| Confirm before destructive actions | FR-HIST-05.                                                          |
| Autosave or save and reopen        | FR-HIST-01, FR-HIST-03.                                              |
| Search and filter                  | FR-RULE-04, FR-HIST-02.                                              |
| Keyboard shortcuts                 | Out of scope: non-interactive CLI.                                   |
| Settings that persist              | FR-CLI-02.                                                           |
| First run and onboarding           | FR-CLI-07, README quick start, crate docs example.                   |
| Offline or bad connection          | FR-CRAWL-06, FR-CRAWL-11. `show`, `history` and `diff` work offline. |
| Permissions per role               | Out of scope: single-user tool.                                      |
| Notifications                      | Out of scope: exit codes and stderr only.                            |
| Import and export                  | FR-REPORT-01 to FR-REPORT-03, FR-HIST-04 baseline import.            |
| Delete and restore                 | FR-HIST-05. No restore.                                              |
| Audit log                          | Out of scope: run history is the record.                             |
| Multi-language and RTL             | Tool text English. FR-TEXT-01, FR-TEXT-03, FR-REPORT-04.             |
| Accessibility                      | FR-REPORT-03, FR-CLI-05.                                             |
| Mobile layout                      | HTML report readable at 360 px (FR-REPORT-03).                       |
| Rate limits and quotas             | FR-CRAWL-01, FR-CRAWL-06, FR-RENDER-03, FR-MEDIA-01.                 |

## 11. Out of scope for MVP

Don't build these even if they look easy.

- Login and auth for staging sites (basic auth, headers, cookies, login forms). Maybe v1.1 as headers plus basic auth.
- MCP server. v1.1, as a thin wrapper over the stable API.
- Roadmap phase 10: backlinks, keyword gaps, index coverage, integrity heuristics (malware patterns, leaked secrets), ad-blocker impact. v2.
- Readability formulas for German, Spanish, French, Italian, Dutch. v1.x.
- Dictionary word segmentation for scripts without word spaces (Chinese, Japanese, Thai).
- Docker image.
- Translated tool text and reports.
- GUI, hosted service, dashboard, scheduled runs. Never in this repo.
- PDF and CSV export.
- Rendered crawl as the default (it stays opt-in).
- A docs website (mdBook or similar).
- Import of TS package reports.

## 12. Top risks and mitigations

1. Browser stage on three OSes (chromiumoxide maintenance, Chrome detection per OS, axe-core and Core Web Vitals over CDP) : M0 spike on every OS. Fallback crate chromey with the same API. If both fail on an OS, stop and revise stage 4.
2. `spawn` inside a non-tokio executor (runtime ownership, shutdown panic) : M0 spike, `shutdown_background()` in `Drop`, the `spawn-futures` example and test kept in CI.
3. scraper's DOM is not `Send`, and memory at 10,000 pages : PageData principle (section 3), M0 memory spike, NFR-02 check before each release.
4. Parity drift while the TS package keeps changing : port from commit `de27aea`. At the end of M2, compare against the TS repo's current HEAD and log differences in `PROGRESS.md` for Safi to decide.
5. Scope (89 Must requirements for one developer) : parity first (M1 to M3, release 0.1), then one roadmap phase per milestone, each releasable.
6. Multi-language text accuracy : fixture tests per language with hand-counted values (English, Arabic, a German or French page, a Chinese page).
7. PSI quotas (25,000 per day, about 400 per 100 s from third-party sources) : cap with `psi_max_pages`, 4 in flight, one retry on 429, then skip with a reason.
8. axe-core MPL-2.0 inside an MIT project : ship the file unmodified with its notices, list it in `THIRD_PARTY.md`, `cargo deny` allows MPL-2.0 for it only.

## 13. Instructions for Claude Code

- Build milestone by milestone, in order. Finish M0 before any feature work.
- Keep a `PROGRESS.md` at the repo root: milestone status, decisions made, deviations from this spec and why.
- At the end of each milestone, run the tests and tick the acceptance boxes it covers. Don't move on with a failing box.
- Don't add features outside this spec. If something seems missing, note it in `PROGRESS.md` and ask.
- If a spec decision turns out to be wrong in practice, stop. Explain why. Don't silently swap a stack choice.
- Commit after each step (each task card finished, each milestone), with a message naming the step. Push to `origin main`.
- Keep secrets out of the repo. Use the env vars in Stage 5 (`SITEVET_HOME`, `SITEVET_PSI_KEY`, `SITEVET_CHROME`, `SITEVET_LOG`, `SITEVET_TEST_CHROME`).
- Track work with the `md-kanban` skill in `docs/sitevet-todo/`, one subfolder per milestone. New cards start as `todo`. Move a card to `in-progress` when you start it, to `done` when finished, or to `review` instead of `done` when Safi needs a visual check (HTML report, CLI output, README, docs).
- Read the TS package at `../Safi-Studio-Scanner` for behaviour, rule logic and fixtures. Never modify anything there.
- Write every doc, README update, rule message and fix text with the `ai-writing-rules:anti-ai-writing` skill. Keep the README honest about what works today.
- Never mention the author's former company brand anywhere in the repo.

---

_Companion files in `docs/spec/`: 01-product, 02-users, 03-requirements, 04-tech-decisions, 05-tech-requirements. This build spec is the entry point._
