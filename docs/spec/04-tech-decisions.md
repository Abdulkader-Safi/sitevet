# Tech decisions

Status: signed off 2026-11-10

For each decision: what was chosen, what else was considered, and why, traced to a requirement (Stage 3) or the mission and users (Stages 1 and 2). Versions are the latest stable on crates.io as of 2026-11-10. Pin minor versions in `Cargo.toml` and let Dependabot raise them.

> **Constraints going in:** Rust only, MIT licence, open source with no budget for paid services. One developer (Safi). The first consumer is a GPUI desktop app in another repo, which runs its own executor. The TS package is the behaviour reference and is never modified. Local toolchain is rustc 1.97.1.
> **Changes:** 2026-11-10: workspace split and the `spawn` entry point were decided here, so FR-DIST-01 and FR-API-06 in stage 3 were updated. 2026-11-13 (M6): `encoding_rs` added for page charsets, and the English stop word list settled. 2026-11-13 (M8): near-duplicates use a 256-bit SimHash, not 64-bit.

## Overall architecture

- **Choice:** a Cargo workspace with two published crates. `crates/sitevet` is the library: crawl, rules, collectors, scoring, rendering, history (behind features). `crates/sitevet-cli` is the binary `sitevet`, a thin layer that parses flags and `sitevet.toml` into `Options`, calls the library, prints progress, and sets exit codes. All audit logic lives in the library.
- **Considered:** one crate with a `cli` feature (rejected: library users would compile clap and SQLite unless they remember `default-features = false`). Three crates with a separate `sitevet-core` (rejected: no consumer needs core without rendering, so it is one more crate to version for nothing).
- **Why:** library-first mission (stage 1), equal primaries (stage 2), NFR-08 dependency weight, FR-DIST-01.

## Language and toolchain

- **Choice:** Rust edition 2024. MSRV is pinned in M0 with `cargo msrv find` against the final dependency set, and must be 1.90 or older so users on a year-old toolchain can build. `rust-toolchain.toml` is not committed for the library, only CI pins versions.
- **Considered:** edition 2021 (rejected: 2024 is stable since Rust 1.85 and every chosen crate supports it).
- **Why:** NFR-06.

## Async runtime and the GPUI entry point

- **Choice:** tokio 1.54 inside the library. Three public entry points:
  1. `audit(url, opts).await` and `audit_stream(url, opts)`, which need a tokio context (FR-API-01, FR-API-02).
  2. `spawn(url, opts) -> AuditHandle`, which runs the audit on a multi-thread tokio runtime sitevet creates lazily and owns (2 worker threads by default, set by `Options::runtime_threads`). The handle exposes events through `async-channel` 2.5 (runtime-agnostic, works under GPUI's executor or `futures::executor`), a `cancel()` method, and an awaitable final report (FR-API-06).
     Cancellation uses `tokio_util::sync::CancellationToken` (tokio-util 0.7.20), passed to every fetch and browser task, and dropping the handle cancels too (FR-API-03).
- **Considered:** tokio-only API with the app using Zed's `gpui_tokio` (rejected by Safi: pushes runtime plumbing into every app). Runtime-agnostic library on `smol` (rejected: reqwest and chromiumoxide need tokio).
- **Why:** FR-API-03, FR-API-06, stage 1 first consumer.
- **Landmines:** Zed's `gpui_tokio` creates a 2-thread runtime and spawns tokio tasks awaited through GPUI's background executor ([Zed source](https://git.secluded.site/zed/blob/main/crates/gpui_tokio/src/gpui_tokio.rs)). Zed hit a panic on exit and fixed it by calling `shutdown_background()` on the owned runtime in `Drop` ([commit 101083e](https://git.secluded.site/zed/commit/101083e7f9c92c52464551d3f888dfa5894b44e8)). sitevet's owned runtime must do the same. Never call `tokio::time::sleep` from code polled by a non-tokio executor: it panics outside a runtime. Community bridge for reference: [gpui-tokio-bridge 0.1.0](https://docs.rs/gpui-tokio-bridge).

## HTTP client

- **Choice:** reqwest 0.13.5 with `rustls` TLS (no OpenSSL, so static musl builds work), `gzip`, `brotli`, `zstd`, `http2`, and manual redirect handling so each hop is recorded (FR-CRAWL-08). One shared `Client` per audit with a per-host semaphore for `concurrency` and a per-host delay (FR-CRAWL-06, NFR-04).
- **Considered:** hyper directly (rejected: more code for the same result). ureq (rejected: blocking).
- **Why:** FR-CRAWL-01, FR-CRAWL-06, FR-CRAWL-08, FR-CRAWL-09.
- **Landmines:** HTTP/3 needs reqwest's unstable `http3` cfg. For FR-PERF-04, detect HTTP/3 from the `alt-svc` response header and the rendered stage's protocol field instead of negotiating it. Record the negotiated version from `Response::version()` for HTTP/2.

## HTML parsing

- **Choice:** scraper 0.29 (html5ever 0.40 underneath, CSS selectors). It is the closest match to cheerio, which the TS rules use, so rule logic ports with the fewest changes (FR-RULE-01). Bytes are decoded to text first with `encoding_rs` 0.8 (the decoder Firefox uses), picking the charset the way browsers do: byte order mark, then the `Content-Type` charset, then a `<meta>` charset in the first 1024 bytes, then UTF-8, with windows-1252 when the bytes are not UTF-8. Without it, pages in windows-1252 or Shift_JIS read as garbled text in the text engine (FR-TEXT-03).
- **Considered:** lol_html 3.0 (streaming rewriter, fast, but no random DOM access, so most rules would need rewriting). tl (fast, but not spec-compliant on broken HTML, which is what audits meet).
- **Why:** FR-RULE-01, FR-MET-01.
- **Landmines:** `scraper::Html` is not `Send`. Parse and run DOM rules inside one synchronous block (`spawn_blocking`), extract everything rules and site-wide checks need into plain `Send` structs, then drop the DOM. Never hold `Html` across an `.await`. This also keeps memory flat for NFR-02: no page keeps its DOM after its rules run.

## Crawl inputs

- **Choice:** `url` 2.5 for parsing and normalising. `texting_robots` 0.2.2 for robots.txt matching (FR-CRAWL-04, FR-AEO-05). `quick-xml` 0.42 for sitemaps and sitemap indexes (FR-CRAWL-03, FR-CRAWL-05).
- **Considered:** robotstxt (port of Google's C++ parser, also fine). Own robots parser (rejected for v1, fallback if needed).
- **Why:** FR-CRAWL-03 to FR-CRAWL-05.
- **Landmines:** texting_robots was last released in March 2023. It is small and passes Google's test suite, but if it fails on a real file, switch to `robotstxt` behind the same internal function.

## Rendered stage (browser)

- **Choice:** chromiumoxide 0.9.1 behind the `browser` cargo feature (FR-RENDER-06), driving the user's installed Chrome or Chromium over the DevTools protocol. Auto-detect the binary per OS, override with `chrome_path`. Inject axe-core 4.14.0 (bundled with `include_str!`) for FR-RENDER-01. Collect Core Web Vitals and timings with `PerformanceObserver` scripts injected before navigation (FR-RENDER-02, FR-PERF-01 to FR-PERF-03). Use DevTools protocol calls for console errors, request timing, viewport emulation and screenshots (FR-MOBILE-01 to FR-MOBILE-04).
- **Considered:** headless_chrome 1.0.22 (rejected: synchronous, thread per browser, so cancellation and streaming fight the async design). chromey 2.58, an actively released fork of chromiumoxide from the spider project (kept as the fallback if chromiumoxide stalls, same API). fantoccini over WebDriver (rejected: needs chromedriver as a second binary). Downloading a pinned Chromium (rejected: about 150 MB per platform, and FR-RENDER-05 already handles a missing browser).
- **Why:** FR-RENDER-01 to FR-RENDER-06, FR-CRAWL-10, FR-PERF, FR-MOBILE, FR-MEDIA-04.
- **Landmines:** chromiumoxide's last release is February 2026 and blog comparisons disagree on its maintenance ([dev.to comparison](https://dev.to/vhub_systems_ed5641f65d59/headless-browsers-in-rust-chromiumoxide-vs-headlesschrome-vs-the-python-alternative-25e5), [docs.rs](https://docs.rs/chromiumoxide)). M0 proves it on all three OSes. Its CDP bindings are generated per protocol revision, so use the crate's default revision, not the latest. Chrome processes must be killed on cancel (FR-API-03), so wrap the browser in a guard that kills on drop.

## Accessibility engine

- **Choice:** axe-core 4.14.0, the same engine the TS package uses through `@axe-core/playwright`, so findings match (FR-RENDER-01).
- **Considered:** none practical in Rust.
- **Why:** parity with the TS package, WCAG 2.2 rules.
- **Landmines:** axe-core is MPL-2.0 ([package.json](https://www.github.com/dequelabs/axe-core/blob/develop/package.json)). MPL is file-level copyleft: ship `axe.min.js` unmodified in its own file with its licence header and `LICENSE-3RD-PARTY.txt`, list it in `THIRD_PARTY.md`, and link its source. The rest of sitevet stays MIT. An axe-core issue warns that bundlers strip its notice comments ([issue 4305](https://redirect.github.com/dequelabs/axe-core/issues/4305)), so never minify or transform the file.

## PageSpeed Insights provider

- **Choice:** call the PSI v5 REST API with reqwest directly, both mobile and desktop strategies optional, capped by `psi_max_pages` (FR-RENDER-03). Map its Lighthouse audits onto the same rule IDs as the local browser.
- **Considered:** a Google API client crate (rejected: one endpoint does not need it).
- **Why:** FR-RENDER-03.
- **Landmines:** quotas are not on an official page in my search. Third-party sources agree on 25,000 queries per day and most say 400 per 100 seconds per key ([Unlighthouse](https://unlighthouse.dev/learn-lighthouse/pagespeed-insights-api/rate-limits), [Screaming Frog guide](https://www.screamingfrog.co.uk/guides/pagespeed-insights-integration)). Each PSI call takes 10 to 30 s, so run them with concurrency 4 and treat HTTP 429 as "retry once after the `Retry-After` delay, then skip with a finding".

## Text engine

- **Choice:** `unicode-segmentation` 1.13 for word and sentence boundaries in any script. `whatlang` 0.18 for language detection when `<html lang>` is missing (FR-TEXT-01). `stop-words` 0.10 for stop word lists (ISO lists, many languages) (FR-TEXT-03). `rust-stemmers` 1.2 (Snowball, 18 languages including Arabic) for word grouping. Readability formulas written in-crate (FR-TEXT-05). N-grams and keyword spans written in-crate (FR-TEXT-06, FR-TEXT-07).
- **Considered:** lingua 1.8 for detection (more accurate on short text, but its language models add tens of MB to the binary, against NFR-08). Per-language lemmatizers (rejected: no maintained Rust option covering many languages, and stemming is enough for keyword grouping).
- **Why:** FR-TEXT-01 to FR-TEXT-08, stage 2 "audits any language".
- **Landmines:** rust-stemmers has not been released since 2019. The Snowball algorithms are stable, so this is acceptable, but vendor it if a bug appears. whatlang is weak on texts under about 20 words, so prefer `<html lang>` and report the detection confidence. Scripts such as Chinese and Thai have no spaces between words, and `unicode-segmentation` does not split them into words. For those, count characters and mark word-based metrics "not available for this language" (see open question 1). The ISO English stop word list has 1,298 entries, including "home", "page", "website" and "information", which would empty the keyword tables of real keywords. English uses a 355-word list instead (found in M6): the TS package's 201 words, the NLTK words it lacks, and the function words from the ISO list (pronouns, prepositions, connectors such as "although" and "therefore"), with its content words left out. Every other language uses the stop-words ISO list.

## Structured data

- **Choice:** `serde_json` for JSON-LD. Microdata and RDFa extraction written in-crate over the scraper DOM, producing the same nested tree type as JSON-LD (FR-AEO-06).
- **Considered:** no maintained Rust microdata or RDFa crate exists.
- **Why:** FR-RULE-01 (structured-data rules), FR-AEO-06, FR-TRUST-02, FR-TRUST-05.

## Site-wide analysis

- **Choice:** in-crate code that runs after the crawl over the extracted page data. Duplicate groups by hash map. Near-duplicates with 256-bit SimHash over word 5-shingles, every pair compared (FR-SITE-02). Link graph with inbound counts and BFS click depth.
- **Landmines:** a 64-bit SimHash (the first choice here) is too noisy for FR-SITE-02. In M8, one changed paragraph in twelve moved it 15 bits, while unrelated pages sat at 33 to 38, so no threshold separated them. 256 bits (each shingle hash stretched into four words with splitmix64) put the same pair 36 bits apart and the unrelated page 134. At 256 bits the bucket trick needs too many bands to help, so every pair is compared: about 50 million XOR-and-popcounts at 10,000 pages, a fraction of a second.
- **Considered:** MinHash with LSH (more accurate, more code, not needed at 10,000 pages).
- **Why:** FR-SITE-01 to FR-SITE-06, NFR-05.

## Tech fingerprinting

- **Choice:** sitevet's own signature file (`data/fingerprints.toml`, embedded with `include_str!`), matched with `regex` 1.13 and `aho-corasick` 1.1 against markup, script URLs, cookies, meta generator tags and headers (FR-TECH-01 to FR-TECH-03).
- **Considered:** Wappalyzer's signature data (rejected: licensing changed away from open source, so it cannot ship in an MIT crate). webappanalyzer community fork (check its licence before borrowing patterns; write sitevet's own entries either way).
- **Why:** FR-TECH-03, NFR-10.

## Media

- **Choice:** `imagesize` 0.15 to read pixel dimensions from the first bytes of a ranged GET (`Range: bytes=0-65535`), falling back to a full GET under 2 MB (FR-MEDIA-01). Optimisation estimates from a per-format heuristic table (FR-MEDIA-02).
- **Considered:** the `image` crate (rejected: decodes whole images, heavy).
- **Why:** FR-MEDIA-01, FR-MEDIA-02, NFR-08.

## Report rendering

- **Choice:** askama 0.16 templates for HTML and Markdown, compiled into the binary, with CSS inlined from one file. JSON through serde. JSON Schema generated from the report types with schemars 1.2 and committed as `schema/report.schema.json` (FR-REPORT-01).
- **Considered:** maud (last release February 2025, and Rust-macro markup is harder for contributors to style). Hand-built strings like the TS package (rejected: the HTML report grows a lot with the new sections).
- **Why:** FR-REPORT-01 to FR-REPORT-04.

## Run history

- **Choice:** rusqlite 0.41 with the `bundled` feature (no system SQLite needed), behind the library's `history` feature, enabled by the CLI (FR-HIST-01). Stored report JSON is zstd-compressed. Paths from `directories` 6 (`ProjectDirs`), overridden by `SITEVET_HOME`.
- **Considered:** sqlx (rejected: async and macro-heavy for a local single-user file). JSON files in a folder (rejected: listing and pruning 1,000 runs gets slow and racy).
- **Why:** FR-HIST-01 to FR-HIST-05.

## CLI

- **Choice:** clap 4.6 with derive. `toml` 1.2 plus serde for `sitevet.toml` with `deny_unknown_fields` and a closest-key suggestion (FR-CLI-02). Colour through `anstream` and `anstyle` (the crates clap uses), which honour `NO_COLOR` (FR-CLI-05). Plain progress lines, no spinner library (FR-CLI-03). `anyhow` for error context in the binary only.
- **Considered:** indicatif progress bars (rejected: FR-CLI-03 wants one line per page, and bars break CI logs).
- **Why:** FR-CLI-01 to FR-CLI-07.

## Errors and logging

- **Choice:** `thiserror` 2.0 for the library's typed errors (FR-CRAWL-11). `tracing` 0.1 for internal logs. The library never installs a subscriber, the CLI installs one at `--verbose`.
- **Why:** library users control their own logging.

## Testing

- **Choice:** `cargo test` with tokio's test macro. A fixture server in `tests/support` that serves folders from `tests/fixtures/sites/<name>/` over a local port with configurable headers, statuses and redirects, built on wiremock 0.6. The TS package's HTML fixtures are copied in (FR-RULE-01). Snapshot tests with insta 1.49 for both report formats and the CLI output. Report schema validation with jsonschema 0.61 in dev-dependencies. Browser tests run only with the `browser` feature and `SITEVET_TEST_CHROME=1`, on Linux CI with Chrome installed.
- **Considered:** httpmock 0.9 (fine too, wiremock is more widely used).
- **Why:** NFR-11, every acceptance line in stage 3 is a test.

## CI and distribution

- **Choice:** GitHub Actions: `fmt`, `clippy -D warnings`, tests on all three desktop OSes, browser tests on Linux. Releases with cargo-dist 0.34: GitHub Releases binaries for the five targets (Linux as static musl), a shell installer and a PowerShell installer, and the Homebrew tap (FR-DIST-02, FR-DIST-03). `cargo publish` for both crates.
- **Considered:** hand-written release workflows (rejected: cargo-dist does all of it from `Cargo.toml` metadata).
- **Why:** FR-DIST-01 to FR-DIST-03.
- **Landmines:** musl builds need rustls (chosen) and the bundled SQLite (chosen). Check that chromiumoxide builds for musl in M0.

## Documentation

- **Choice:** docs.rs for the API, the README, `CONTRIBUTING.md` (FR-EXT-02), and a generated `docs/rules.md` from `sitevet rules --format md`. No separate docs site in v1.
- **Considered:** mdBook site mirroring the TS docs site (later, once the API is stable).
- **Why:** one developer, keep docs next to code.

## Layers that do not apply

Database server, auth, file storage, email, payments, search, analytics, background job queue: none. sitevet is a local library and CLI with no server.

## Reused from existing work

- The TS package at `../Safi-Studio-Scanner` (read only): rule logic in `src/rules/*.ts`, collectors in `src/metrics/*.ts`, scoring in `src/runner.ts` and `src/report.ts`, event shapes in `src/types.ts`, text engine work in `src/text.ts`, HTML fixtures in `test/fixtures/`, and the backlog in `features.md`.
- Zed's `gpui_tokio` pattern for the owned runtime and its shutdown fix.

## Cost

$0 per month. GitHub Actions is free for public repos, crates.io is free, PSI keys are free within quota. No hosted services.

## Licensing

sitevet is MIT. Dependencies checked for MIT compatibility:

| Dependency                                                                                                                       | Licence                                                          |
| -------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| tokio, reqwest, scraper, clap, serde, url, regex, aho-corasick, quick-xml, askama, schemars, thiserror, tracing, insta, wiremock | MIT or MIT/Apache-2.0                                            |
| rusqlite (bundled SQLite)                                                                                                        | MIT, SQLite is public domain                                     |
| chromiumoxide                                                                                                                    | MIT/Apache-2.0                                                   |
| whatlang                                                                                                                         | MIT                                                              |
| stop-words, rust-stemmers, unicode-segmentation, imagesize, directories                                                          | MIT or MIT/Apache-2.0                                            |
| encoding_rs                                                                                                                      | (Apache-2.0 or MIT) and BSD-3-Clause                             |
| texting_robots                                                                                                                   | MIT/Apache-2.0                                                   |
| axe-core (bundled JS)                                                                                                            | MPL-2.0, shipped as an unmodified separate file with its notices |

Verify at build time with `cargo deny check licenses` in CI, allowing MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0 and MPL-2.0 (axe only).

## Top technical risks

1. **Browser stage on three OSes** : chromiumoxide maintenance is uncertain, Chrome detection differs per OS, and axe injection plus CWV collection must work through CDP. Spike in M0: render one page, run axe, read LCP and CLS, take a screenshot, cancel mid-run and confirm no Chrome process is left, on every desktop OS. Fallback: swap to chromey (same API).
2. **`spawn` inside a non-tokio executor** : runtime ownership and clean shutdown on cancel, the exact problem Zed hit. Spike in M0: call `spawn` from `futures::executor::block_on`, stream events, cancel, drop, and check no panic on exit.
3. **scraper is not `Send` and memory at 10,000 pages** : spike in M0: parse 1,000 fixture pages through `spawn_blocking` with extraction, and measure resident memory against NFR-02.
4. **Parity drift** : the TS package is still changing. Port rules from the TS source at a recorded commit hash, and re-check that hash at the end of the parity milestone.
5. **Scope** : 89 Must requirements across 9 roadmap phases for one developer. Milestones ship parity first, then phases in roadmap order, each releasable as a 0.x version, so a stall leaves something usable.
6. **Multi-language text accuracy** : Arabic tokenizing and stemming, and scripts without spaces. Fixture tests per language with hand-counted values.

---

### Open questions

None.

### Assumptions confirmed

1. **Crate layout : RESOLVED.** Workspace with `sitevet` (library) and `sitevet-cli` (binary `sitevet`). CLI installs with `cargo install sitevet-cli`.
2. **Runtime : RESOLVED.** Async tokio API plus `spawn()` on an owned runtime with runtime-agnostic events and cancel, for GPUI and other non-tokio apps.
3. **Readability languages : RESOLVED.** English formulas only in v1 (Flesch reading ease, Flesch-Kincaid, Gunning Fog). Every other language gets the neutral fallback. German, Spanish, French, Italian and Dutch adaptations are a v1.x add. FR-TEXT-05 in stage 3 updated.
4. **Scripts without word spaces : RESOLVED.** Chinese, Japanese, Thai and similar get character counts, with word-based metrics marked "not available for this language".
5. **MSRV : RESOLVED.** Pinned in M0, no newer than Rust 1.90.
