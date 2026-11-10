# Technical requirements

Status: draft

The engineering blueprint. Tech context: Rust 2024 workspace (`sitevet` library, `sitevet-cli` binary), tokio 1.54, reqwest 0.13 with rustls, scraper 0.29, chromiumoxide 0.9 behind `browser`, rusqlite 0.41 behind `history`, askama 0.16, clap 4.6.

## Architecture

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

The keystone is `PageData`: a plain, `Send`, serialisable struct extracted from each page's DOM in one blocking step. Everything after the DOM stage (link checks, the rendered stage, site-wide rules, history, rendering) reads `PageData`, never the DOM. That one structure is what lets scraper's non-`Send` DOM live inside a single blocking call, keeps memory flat at 10,000 pages (NFR-02), and gives site-wide rules (phase 4) one shape to compare across pages. Rules are grouped by the stage they need, so each runs as soon as its inputs exist and progress events flow per page.

## Data models

Conventions:

- Rust structs with `serde` derive. JSON field names are `snake_case`.
- Time: RFC 3339 UTC strings in JSON (`2026-11-10T09:30:00Z`). Durations: integer milliseconds, field names ending `_ms`. Sizes: integer bytes, field names ending `_bytes`. Renderers format them for people.
- IDs: rule and metric IDs are `category/slug` strings, the same as the TS package. History run IDs are SQLite integers (short to type: `sitevet show 12`).
- Enums serialise as lowercase strings.
- Deletion is hard delete (history only). No tenancy.
- Every public enum and struct that may grow is `#[non_exhaustive]`, so adding a field or variant is not a breaking change (NFR-09).

### Options

| Field | Type | Notes |
|-------|------|-------|
| only | Vec<String> | categories to include, empty = all |
| skip | Vec<String> | categories to skip |
| max_pages | u32 | default 100, max 10,000 |
| max_depth | u32 | default 5 |
| concurrency | u32 | per host, default 5 |
| delay_ms | u64 | per-host delay, default 0 |
| timeout_ms | u64 | per request, default 15,000 |
| user_agent | Option<String> | default `sitevet/<ver> (+https://github.com/Abdulkader-Safi/sitevet)` |
| ignore_robots | bool | default false |
| crawl_sitemap | bool | default true |
| allow_private_network | bool | default: true when the start URL is private or localhost, else false (see Security) |
| browser | bool | needs `browser` feature, default false |
| chrome_path | Option<PathBuf> | default auto-detect, env `SITEVET_CHROME` |
| render_crawl | bool | default false |
| psi_key | Option<String> | env `SITEVET_PSI_KEY`, never serialised into reports or history |
| psi_max_pages | u32 | default 5 |
| media | bool | default false |
| media_max_requests | u32 | default 200 |
| screenshots | bool | default false, needs browser |
| screenshot_dir | Option<PathBuf> | default next to `--out`, or the current folder |
| click_depth_warn | u32 | default 3 |
| slow_resource_ms | u64 | default 300 |
| runtime_threads | usize | `spawn` only, default 2 |
| rules | Option<Vec<Arc<dyn Rule>>> | library only, replaces the registry; skipped by serde |
| collectors | Option<Vec<Arc<dyn Collector>>> | library only; skipped by serde |

`sitevet.toml` keys are the serde names of these fields, except the two library-only ones. CLI-only settings (`format`, `out`, `fail_under`, `fail_on`, `baseline`, `save`) live in a `[cli]` table in the same file.

### AuditReport

| Field | Type | Notes |
|-------|------|-------|
| schema_version | u32 | 1 for v1.0 |
| sitevet_version | String | crate version |
| start_url | String | as given |
| final_url | String | after redirects |
| generated_at | String | RFC 3339 |
| duration_ms | u64 | |
| options | OptionsSummary | the options used, with `psi_key` removed |
| score | u8 | overall, 0 to 100 (FR-SCORE-01) |
| categories | Vec<CategoryScore> | sorted by category |
| pages | Vec<PageResult> | crawl order |
| site | SiteResult | site-wide findings and metrics (phase 4) |
| blocked | Vec<BlockedUrl> | URLs skipped by robots.txt, with the matching rule |
| skipped_rules | Vec<SkippedRule> | rule ID and reason, e.g. "needs browser or psi_key" |
| changes | Option<ReportDiff> | only when a baseline was given (FR-HIST-04) |

### CategoryScore

| Field | Type | Notes |
|-------|------|-------|
| category | String | |
| score | u8 | `score_findings` over all findings in the category |
| pass, fail, warn | u32 | counts |

### PageResult

| Field | Type | Notes |
|-------|------|-------|
| url | String | normalised |
| final_url | String | |
| status | u16 | 0 when the fetch failed |
| error | Option<String> | fetch error, if any |
| depth | u32 | click depth from start |
| redirect_chain | Vec<String> | each hop |
| response_ms | u64 | |
| html_bytes | u64 | |
| truncated | bool | true when HTML exceeded 10 MB (NFR-05) |
| language | LanguageInfo | declared, detected, confidence (FR-TEXT-01) |
| score | u8 | `score_findings` over this page |
| findings | Vec<Finding> | |
| metrics | Vec<Metric> | |
| screenshots | Vec<Screenshot> | empty unless enabled |

### Finding

| Field | Type | Notes |
|-------|------|-------|
| rule_id | String | `core-seo/title-present`, or an override such as `accessibility/label` |
| category | String | |
| title | String | |
| status | Status | pass, fail, warn, info |
| severity | Severity | error, warning, info |
| message | String | English |
| details | Option<String> | |
| evidence | Option<String> | offending markup, header or value, capped at 2,000 chars |
| fix | Option<String> | |
| key | String | stable identity for diffs: hash of rule_id + page URL + evidence. Not shown to people |

### Metric

| Field | Type | Notes |
|-------|------|-------|
| id | String | `content/word-count` |
| category | String | |
| label | String | |
| status | Option<ValueStatus> | ok, warn, bad, missing. Never scored |
| note | Option<String> | |
| body | MetricBody | `Value { value: MetricCell or null, unit: Option<String> }` or `Table { columns, rows, total }` |

`MetricCell` is a string or a number. Tables keep at most 100 rows (`ROW_CAP`, same as the TS package if it differs, the TS value wins), with `total` before the cap (FR-MET-02).

### SiteResult

| Field | Type | Notes |
|-------|------|-------|
| findings | Vec<SiteFinding> | a Finding plus `urls: Vec<String>` it affects (duplicate group, orphans, redirect map entries) |
| metrics | Vec<Metric> | link graph table, click depth table, sitemap reconciliation, duplicate groups |

### PageData (internal, not in the report)

The extraction struct every post-DOM stage reads. Fields: url, final URL, status, headers, timings, title, meta tags map, canonical, robots directives, hreflang list, headings outline, links (href, absolute URL, anchor text, rel, internal flag, in-main-content flag), images (src, alt, width, height, loading, srcset), scripts (src, inline size, async/defer), stylesheets, structured data tree (JSON-LD, microdata, RDFa), main-content text, body text, language info, forms count, iframes, cookies from headers, fingerprint hits, raw-HTML content hash and SimHash. Rendered data (axe results, CWV, timings, console errors, rendered text, screenshot paths) is added when the rendered stage runs.

### AuditEvent

| Variant | Fields |
|---------|--------|
| AuditStart | url, max_pages |
| PageFetched | url, status, depth, index (1-based), ms |
| CrawlComplete | pages, checks_total, ms |
| RenderComplete | provider (browser or psi), rendered, pages, ms |
| CheckComplete | url, rule_id, status, checks_done, checks_total |
| PageComplete | url, index, pages_total, score, findings, metrics, ms |
| AuditComplete | score, pages, checks, ms, report (boxed) |

Same names and fields as the TS events, in snake_case. All variants are `Send + Clone + Serialize` (FR-EVENT-02).

### History run (SQLite)

Table `runs`:

| Field | Type | Notes |
|-------|------|-------|
| id | INTEGER | primary key, autoincrement |
| start_url | TEXT | indexed |
| origin | TEXT | indexed, for `history <url>` filtering by site |
| started_at | TEXT | RFC 3339, indexed |
| duration_ms | INTEGER | |
| pages | INTEGER | |
| score | INTEGER | |
| sitevet_version | TEXT | |
| schema_version | INTEGER | report schema at save time |
| options_json | TEXT | options summary without secrets |
| report_zst | BLOB | zstd-compressed report JSON |

Schema version in `PRAGMA user_version`. Database file `history.db` under the data folder.

### ReportDiff

| Field | Type | Notes |
|-------|------|-------|
| baseline | BaselineRef | run ID or file path, and its date |
| score_change | i16 | overall |
| categories | Vec<CategoryChange> | category, before, after |
| new_failures | Vec<FindingRef> | keys failing now that passed or did not exist before |
| fixed | Vec<FindingRef> | keys failing before that pass or are gone now |
| pages_added, pages_removed | Vec<String> | |

### Bundled data

- `data/fingerprints.toml` : vendor, category (analytics, ads, cms, framework, cdn), and match patterns per source (html, script_src, header, cookie, meta).
- `data/authoritative-domains.txt` : one domain suffix per line for FR-TRUST-04.
- `assets/axe.min.js` and its licence files, unmodified.
- `assets/report.css` : the HTML report stylesheet, inlined at render time.

## API / command surface

Style: a Rust library API, plus a CLI that maps onto it. Library errors are a `sitevet::Error` enum (thiserror): `InvalidUrl`, `InvalidOptions { field, reason }`, `StartPageUnreachable { url, source }`, `NotHtml { url, content_type }`, `BrowserNotFound { tried }`, `BrowserFailed`, `History`, `Cancelled`. Per-page problems never become errors: they become findings or `PageResult.error` (FR-CRAWL-11). No auth anywhere.

### Library: audit

- `audit(url, Options) -> Result<AuditReport>`: full audit in a tokio context (FR-API-01). Long-running.
- `audit_stream(url, Options) -> AuditStream`: `impl Stream<Item = AuditEvent>`, plus `final_report().await` (FR-API-02, FR-EVENT-01). Dropping it cancels (FR-API-03).
- `spawn(url, Options) -> AuditHandle`: runs on sitevet's owned runtime. `handle.events() -> async_channel::Receiver<AuditEvent>`, `handle.cancel()`, `handle.report().await -> Result<AuditReport>` on any executor. Drop cancels (FR-API-06).
- `audit_to_html(url, Options)`, `audit_to_markdown(url, Options)`, `audit_score(url, Options)`: one-call helpers (FR-API-05).
- `Options::default()`, `Options::from_toml_str(&str) -> Result<Options>`, `Options::validate()`: FR-API-04, FR-CRAWL-02, FR-CLI-02.

### Library: render and inspect

- `render(&AuditReport, Format) -> String` with `Format::{Json, Markdown, Html}` (FR-API-05, FR-REPORT-01 to FR-REPORT-04). HTML takes `RenderOptions { screenshot_dir }` through `render_with`.
- `diff(&AuditReport, &AuditReport) -> ReportDiff` (FR-HIST-04).
- `all_rules() -> Vec<Arc<dyn Rule>>`, `all_collectors()`, `select_rules(&Options)` (FR-EXT-01, FR-CLI-06).
- `report_schema() -> &'static str`: the JSON Schema (FR-REPORT-01).

### Library: extending

```rust
pub enum Stage { Dom, Enriched, Rendered }

pub struct RuleMeta { pub id: &'static str, pub category: &'static str, pub title: &'static str,
                      pub severity: Severity, pub stage: Stage, pub fix: Option<&'static str> }

pub trait Rule: Send + Sync {
    fn meta(&self) -> RuleMeta;
    fn run(&self, cx: &RuleCx<'_>) -> Vec<RuleResult>;
}

pub trait SiteRule: Send + Sync {          // phase 4, runs once after the crawl
    fn meta(&self) -> RuleMeta;
    fn run(&self, cx: &SiteCx<'_>) -> Vec<SiteRuleResult>;
}

pub trait Collector: Send + Sync { fn meta(&self) -> CollectorMeta; fn collect(&self, cx: &RuleCx<'_>) -> Vec<MetricInput>; }
pub trait SiteCollector: Send + Sync { /* same, over SiteCx */ }
```

`RuleCx` gives `page: &PageData`, `dom: Option<&Html>` (only in the `Dom` stage), `links: &LinkStatusMap` (from `Enriched` on), `rendered: Option<&RenderedData>` (only in `Rendered`), and `site: &SiteData` (robots, sitemap). Rules are synchronous: every network result they need is fetched before their stage starts. This replaces the TS package's async `checkUrl` calls inside rules. A rule that needs links declares `Stage::Enriched`, and the link checker resolves all of a page's links first (FR-CRAWL-09). Errors: a rule that panics is caught, reported as an `info` finding "rule crashed", and the audit continues.

### History (feature `history`)

- `History::open(path: Option<PathBuf>) -> Result<History>`: default path from `SITEVET_HOME` or the OS data folder.
- `history.save(&AuditReport) -> Result<RunId>` (FR-HIST-01).
- `history.list(origin: Option<&str>, limit) -> Result<Vec<RunSummary>>` (FR-HIST-02).
- `history.load(RunId) -> Result<AuditReport>` (FR-HIST-03).
- `history.delete(RunId)`, `history.prune(keep: usize) -> Result<usize>` (FR-HIST-05).
- `history.latest_for(origin) -> Result<Option<RunId>>`: used for `--baseline last`.

### CLI

- `sitevet audit <url> [flags]`: one flag per `Options` field (kebab-case), plus `--format`, `--out`, `--config`, `--fail-under <0-100>`, `--fail-on <error|warning>`, `--baseline <id|last|file.json>`, `--no-save`, `--quiet`, `--plain`, `--verbose` (FR-CLI-01, FR-CLI-04, FR-CLI-05, FR-HIST-04). Exit 0, 1 or 2.
- `sitevet history [url] [--limit n]` (FR-HIST-02), **`sitevet history delete <id> [--yes]`**, **`sitevet history prune --keep <n> [--yes]`** (FR-HIST-05).
- `sitevet show <id> [--format] [--out]` (FR-HIST-03).
- `sitevet diff <id-a> <id-b> [--format]` (FR-HIST-04).
- `sitevet rules [--category c] [--format table|md|json]` (FR-CLI-06).
- `sitevet init [--force]` (FR-CLI-07). Refuses to overwrite without `--force`.
- `sitevet --version`, **`sitevet help`**.

## Permissions

Single-user tool, no roles. Authorisation is the operating system's: the CLI reads and writes files the user can, and the library runs with the host app's rights.

## Screens and key components

sitevet has two visible surfaces: CLI terminal output and the HTML report.

### CLI: audit run

- **Purpose:** show progress and the result of `sitevet audit` (FR-CLI-03).
- **Components:** a start line with URL and limits. One line per page as it completes: `3/100  https://site/about  92/100`, with the score coloured and a status word. A summary block: overall score, per-category score with fail and warn counts, the number of blocked URLs and skipped rules, the report path, and the history run ID.
- **States:** loading (crawl lines appear as pages finish, a "crawling..." line until `CrawlComplete`), error (one red `error:` line with the cause and a hint, exit 2), empty (start page has zero links: "1 page audited, no internal links found"), success (summary, exit 0 or 1).

### CLI: history, show, diff, rules

- **Purpose:** FR-HIST-02 to FR-HIST-05, FR-CLI-06.
- **Components:** aligned plain-text tables. `diff` shows the overall score change, category changes, then new failures and fixed findings as lists.
- **States:** empty ("No runs saved yet. Run sitevet audit <url>."), error (unknown run ID, exit 2), success.

### HTML report

- **Purpose:** the client-ready deliverable (FR-REPORT-03, FR-REPORT-04).
- **Components:** header with the audited URL and date next to the overall score badge. "Changes since last run" panel when a baseline exists. Category summary grid (score plus a count per status). Site-wide section (duplicate groups, orphans, link graph, redirect map, sitemap reconciliation, tech stack). Per-page collapsible sections with findings grouped by category (status word plus colour, message, evidence in a code block, fix text) and metric tables with "showing N of M". Screenshot thumbnails per page when enabled. Footer with sitevet version and schema version.
- **States:** empty category ("No checks ran in this category"), skipped rules listed with their reason, failed page (status and error instead of findings).
- **Constraints:** one file, inline CSS, no JavaScript needed to read it (collapsible sections use `<details>`), no external requests, WCAG 2.2 AA, readable at 360 px, quoted page text wrapped with `dir="auto"`. Neutral styling: a light and dark theme through `prefers-color-scheme`, system fonts.

### Markdown report

- **Purpose:** paste into issues and docs (FR-REPORT-02).
- **Components:** score summary table, category tables, site-wide section, per-page findings tables, metric tables. Pipe characters and newlines in cells are escaped.

## State management

- The library holds no global state except the lazily created runtime used by `spawn`, which shuts down with `shutdown_background()` when the last handle drops.
- Each audit owns its state: crawl queue, seen-URL set, link-status cache, per-host semaphores, `Vec<PageData>`, and results. Shared across tasks with `Arc` and `tokio::sync::Mutex` only where needed (the link cache is one `Mutex<HashMap<Url, Shared<Future>>>`, so concurrent callers await the same check).
- The caller hears about changes only through `AuditEvent`s. Events are sent with backpressure: if the consumer stops reading, the bounded channel (capacity 1,024) blocks producers instead of growing memory.
- Undo and redo: not applicable.
- Persistence: only the history database and the files the user asks for (report, screenshots).
- The CLI is stateless between runs apart from `history.db` and `sitevet.toml`.

## Background and long-running tasks

- Crawl: triggered by any entry point. Breadth-first queue, `concurrency` permits per host, delay per host. Emits `PageFetched` per page and `CrawlComplete` with totals. Retries: one retry on connection reset or 502/503/504 with a 1 s wait, none on 4xx. Failure on the start URL is an error, elsewhere a page result.
- Link checking: HEAD first, GET on 405 or 501, at most `concurrency` per host, 10 redirects max, results cached per URL for the audit. External links share a global cap of 20 in flight.
- Rendered stage: one browser per audit, one tab per page, up to 2 tabs at once (pages are heavy). Per-page timeout of 30 s. A crashed tab becomes a page finding, a crashed browser is restarted once, then the stage is marked failed and its rules are skipped with that reason. Emits `RenderComplete`.
- PSI: 4 requests in flight, 60 s timeout each, retry once after `Retry-After` on 429, then a skipped-with-reason result.
- Media probes: ranged GET, 8 in flight, stop at `media_max_requests`.
- Site-wide analysis: runs once after all pages finish. CPU-bound, so it runs in `spawn_blocking`.
- History save: after `AuditComplete`, in the CLI. A failed save prints a warning and does not change the exit code.
- Cancellation: one `CancellationToken` per audit, checked by every task before each request and passed into `select!` around every await. On cancel: abort in-flight requests, close browser tabs, kill the Chrome process, return `Error::Cancelled` within 1 second (FR-API-03).

## Security

- **Untrusted input everywhere.** Anything the site sends (HTML, headers, robots files, sitemaps) is attacker-controlled. Parsing must not panic: rules run under `catch_unwind`, and every size is capped (HTML 10 MB, robots.txt 500 KB, sitemap 50 MB and 50,000 URLs per file, evidence 2,000 chars, JSON-LD block 1 MB).
- **Decompression bombs:** cap decoded body size at the same limits, not just the transfer size.
- **SSRF:** a public site can link to `http://169.254.169.254/` or `http://localhost:6379/`. When the start URL resolves to a public address, sitevet refuses to request private, loopback, link-local and metadata addresses (checked after DNS resolution, so DNS rebinding is covered), unless `allow_private_network` is true. When the start URL is itself private (auditing `localhost:3000`), private addresses are allowed by default. This matters most for apps that host sitevet as a service.
- **Report injection:** all page-derived text in the HTML report is escaped by askama's auto-escaping. No `|safe` filter on page data. Markdown escapes page text, so pipes, backticks, asterisks or angle brackets cannot break the layout. Screenshots are embedded as `data:image/png;base64` only.
- **Browser isolation:** a fresh temporary profile per audit, deleted afterwards. Chrome's sandbox stays on. `--no-sandbox` is only added when the process runs as root on Linux (common in CI containers), and a warning says so. Downloads and permission prompts are denied through CDP.
- **Secrets:** `psi_key` comes from the option or `SITEVET_PSI_KEY`. It is never written into reports, history, events, logs or error messages, and the PSI request URL is redacted in logs.
- **Paths:** `--out` and `screenshot_dir` are written as given. Screenshot file names are derived from a hash of the page URL, never from the URL text, so a page cannot write outside the folder.
- **Dependencies:** `cargo deny` in CI for licences and RustSec advisories.
- **Politeness and abuse:** the default user agent identifies sitevet with the repo URL, robots.txt is obeyed by default, and concurrency per host is capped.

## Logging and observability

- The library logs with `tracing` at debug level for each request (method, URL with query string removed, status, ms) and at warn level for retries and skipped stages. It never installs a subscriber.
- The CLI installs a subscriber at `--verbose` (debug) or with `SITEVET_LOG=<filter>`. Logs go to stderr.
- Never logged: `psi_key`, cookie values, `Authorization` headers, full query strings.
- No telemetry, no crash reporting (NFR-03).

## Testing strategy

- **Unit tests** next to the code for URL normalisation, robots matching, scoring (the TS scoring cases ported exactly), SimHash, readability formulas, text tokenizing per language, fingerprint matching, diff logic, option validation.
- **Rule and collector tests:** one test module per category. Each rule has a passing and a failing fixture (NFR-11). The TS fixtures are copied into `crates/sitevet/tests/fixtures/pages/`.
- **Integration tests** in `crates/sitevet/tests/` against fixture sites served by the support server (`tests/fixtures/sites/<name>/`, with a `site.toml` that sets headers and statuses per path, plus redirects): crawl limits, robots obeying, sitemap seeding, redirects, dedup, cancellation timing, events order, site-wide findings, SSRF blocking, size caps.
- **`spawn` test:** drives `spawn` from `futures::executor::block_on` with no tokio context (FR-API-06).
- **Browser tests:** behind `--features browser` and `SITEVET_TEST_CHROME=1`, run on Linux CI with Chrome installed, and locally on demand.
- **PSI tests:** recorded responses replayed by the fixture server. No live calls in CI.
- **Snapshot tests (insta):** Markdown and HTML for a fixed fixture report, CLI stderr for `audit`, `history`, `diff`, `rules`.
- **Schema test:** every report produced in the test suite validates against `schema/report.schema.json`, and the committed schema equals the generated one.
- **CLI tests** in `crates/sitevet-cli/tests/` with `assert_cmd`: exit codes, `NO_COLOR`, TOML errors, `--no-save`.
- **Performance check:** an ignored-by-default benchmark test runs a 100-page fixture site and a 1,000-page memory check for NFR-01 and NFR-02. Run before each release.
- **Run:** `cargo test --workspace`, `cargo test --workspace --all-features` with Chrome available, `cargo test -p sitevet <name>` for one test.

## Environment and configuration

- `SITEVET_HOME` : folder for `history.db`. Default: the OS data folder from `directories` (`~/Library/Application Support/sitevet`, `~/.local/share/sitevet`, `%APPDATA%\sitevet`).
- `SITEVET_PSI_KEY` : PageSpeed Insights key. Default unset.
- `SITEVET_CHROME` : path to the Chrome binary. Default auto-detect.
- `SITEVET_LOG` : tracing filter for the CLI, e.g. `sitevet=debug`. Default off.
- `NO_COLOR` : disables colour (FR-CLI-05).
- `SITEVET_TEST_CHROME` : `1` enables browser tests. Test only.

No secrets in the repo. `sitevet.toml` may hold `psi_key`, so `sitevet init` writes it commented out with a note to prefer the environment variable.

## Migrations and seed data

- **History database:** migrations are embedded SQL files applied in order on `History::open`, tracked with `PRAGMA user_version`. v1 ships migration 1. A database from a newer sitevet is opened read-only with a warning.
- **Report schema:** `schema_version = 1`. `show` and `diff` read stored reports with serde defaults for fields added later. A stored report with a higher `schema_version` than the running binary prints a warning and shows only the summary columns from the `runs` table.
- **Seed data:** none. Fixture sites in the repo are the demo data, and `cargo run -p sitevet-cli -- audit http://127.0.0.1:<port>` against `examples/serve-fixture` gives a demo run.
- **Import:** no import from the TS package's JSON. `--baseline file.json` accepts sitevet reports only.

---

### Open questions

1. Site-wide findings and the score: should site findings (duplicate titles, orphans, redirect map entries) count in the category scores and the overall penalty? : recommend yes, each site finding counts once in its category and adds to the overall penalty scaled by the share of pages it affects (the same idea as the TS overall formula).
2. SSRF default: block private addresses when the start URL is public, allow them when the start URL is private. OK? : recommend yes.
3. History run IDs as short integers (`sitevet show 12`), not ULIDs. OK? : recommend yes.
4. Table row cap of 100 rows per metric table (TS value wins if different). OK? : recommend yes.

### Assumptions confirmed

1. **Rule shape : RESOLVED.** Synchronous rules grouped by stage, reading `PageData` (and the DOM only in the `Dom` stage). Network data is fetched before the stage runs. Decided from stage 4's scraper constraint.
2. **Time and units : RESOLVED.** RFC 3339 UTC strings, integer milliseconds and bytes in JSON.
3. **Deletion : RESOLVED.** Hard delete, history only (FR-HIST-05).
