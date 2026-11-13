# Progress

Build log for sitevet, kept per section 13 of `docs/spec/06-build-spec.md`.

## Milestone status

| Milestone      | Status                                   |
| -------------- | ---------------------------------------- |
| M0 de-risk     | Done. Safi gave the go on 2026-11-10       |
| M1 engine core | Done. CI green on 2026-11-10 (run 42813274176). README in review |
| M2 parity rules | Done. CI green (run 42827541672). CONTRIBUTING.md and the TS diff in review |
| M3 reports, CLI | Done. 0.1.0 released 2026-11-10 (crates.io and GitHub). Reviews closed |
| M4 rendered stage | Done. CI green (run 42866636468) |
| M5 run history | Done. 0.2.0 released 2026-11-10, 0.2.1 the same day with the diff fix (PR #2), 0.2.2 on 2026-11-11 with render progress and the LinkedIn 999 fix (PR #3), 0.2.3 on 2026-11-13 with the HEAD-then-GET link fix (PR #4) |
| M6 text engine | Built on branch `m6/text-engine`, PR #5 open, not merged |
| M7 answer engine | Built on branch `m7/answer-engine` (stacked on M6), PR open, not merged |
| M8 to M14      | Not started                              |

## M7: answer engine and content structure

Built on the branch `m7/answer-engine`, which starts from the M6 branch because it reads the same main-content boundary. The TS package lists phase 3 in its backlog without code, so the logic and thresholds here are sitevet's own. Every measurement is a metric in a new "Answer engine" category and never moves the score. The two new rules are `structured-data/faqpage-valid` and `structured-data/howto-valid`.

### Acceptance boxes

- [x] FR-AEO-01: `answers.html` has a 6-word paragraph, then a 40-word definition. The report says block 2, 40 words.
- [x] FR-AEO-02: the same fixture has 4 question H2s, 3 followed by a short paragraph and one by a list. Coverage is 0.75.
- [x] FR-AEO-03: hand-counted on that fixture: 1 list, 1 step list, 1 table, 2 definition blocks (a `<dl>` and a "Humus: ..." paragraph), 1 FAQ block, 5 H2 sections of 7, 11, 8, 7 and 20 words (average 10.6, outside the 100 to 300 band).
- [x] FR-AEO-04: `topic.html` has links in three places outside the main content. Only the 3 main-content internal links count (2 unique). The aside's "Related articles" block is found with 2 links, there are 2 distinct H2s next to 1 H3, and 3 of 4 anchors are descriptive.
- [x] FR-AEO-05: a robots.txt that disallows GPTBot reports GPTBot blocked and ClaudeBot, PerplexityBot, CCBot and Google-Extended allowed. `llms.txt` is fetched once per audit and parsed. A page whose raw HTML has no text but loads a script warns.
- [x] FR-AEO-06: a microdata FAQPage and its JSON-LD twin give the same tree, and an RDFa Article matches its JSON-LD twin too. FAQPage and HowTo rules pass, warn or fail on fixtures in JSON-LD and microdata.
- [x] FR-AEO-07 (Should): `article.html` is labelled article.

### Decisions made in M7

- The thresholds, each a named constant in `metrics/answer.rs`: an answer near the top is a paragraph of 15 to 80 words in the first 4 blocks, and on English pages it must read as a definition ("X is ...", "means", "refers to"). A short answer under a question heading is a paragraph of up to 60 words. The section band is 100 to 300 words. Under 50 raw words on a page with scripts suggests content that needs JavaScript.
- Question headings end in a question mark (Latin, Arabic or full-width), or on English pages start with a question word.
- Sections are read in document order: everything between one H2 and the next, however the markup nests. The first version read the H2's siblings, and the live run showed 0-word sections on abdulkadersafi.com, which wraps each H2 in its own `<div>`.
- Page type checks run in this order: the site root is home, then Product, Article or ContactPage schema, then a contact URL or H1, then article structure, then business schema. Business schema often sits on every page of a site; checked first, it labelled every page of abdulkadersafi.com a local business.
- AI crawler access is per page, checked against the page's own URL, like the other robots.txt metrics.
- The schema tree marks a node "incomplete" when it lacks the key properties of a short per-type list (Article, Organization, LocalBusiness, Product, FAQPage, HowTo, and the types nested under them). Other types are listed without a check.
- With `--render-crawl`, the DOM is the one Chrome built, so "content without JavaScript" says it was not measured. `PageData` gets `js_rendered`, `schema` and an `in_main` flag on each link, and `SiteData` gets `llms`.
- With no FAQPage or HowTo on a page, the two new rules return info, which the score skips, so pages without that markup score as before.
- A new `answer-engine` category gets the label "Answer engine" and sorts after the TS categories.

## M6: text engine

Built on the branch `m6/text-engine`. Every page now gets word counts and readability in its own language, with keyword tables, and `PageResult` carries a `language` field with the declared and detected language.

### Acceptance boxes

- [x] FR-TEXT-01: `text-ar.html` has no `lang` attribute and is detected as `ar`. `PageResult.language` holds the declared value, the detected one and the confidence.
- [x] FR-TEXT-02: `boilerplate.html` repeats "pizza" 50 times in its footer, and "pizza" is not in the top keywords ("bread" is first).
- [x] FR-TEXT-03: the English fixture gives 20 words and 4 stop words, the Arabic one 17 words and 4 stop words (23.5%), both hand-counted against the lists in use.
- [x] FR-TEXT-04: the English fixture matches the hand-computed values exactly: 14 unique words (70%), 20% stop words, 20% water, 5 words per sentence, 97 characters.
- [x] FR-TEXT-05: the English fixture gives Flesch 83.3 ("Easy"), Flesch-Kincaid 2.9 and Gunning Fog 4, worked out by hand with the TS syllable estimate. The Arabic and French fixtures show "Not available for this language" for all three, and the Chinese one counts 23 characters with every word metric marked not available.
- [x] FR-TEXT-06: "garden soil" is first among the English fixture's 2-word phrases. The stop word table lists each stop word with its count.
- [x] FR-TEXT-07: the relevance matrix marks each top keyword in the title, description and H1, and the heading spans are byte ranges (`0-6 7-11` for "Garden soil" in the title, `18-22` for "soil" in the description).
- [x] FR-TEXT-08: `content/keyword-stuffing` reads the n-gram output. The stuffed test page warns and the varied one passes.

### Decisions made in M6

- English uses a 355-word stop word list. The ISO English list from `stop-words` has 1,298 entries, including "home", "page", "website", "best" and "information", which would empty the keyword tables of real keywords. Safi asked for more stop words than the TS package's 201, so the list is those 201, the NLTK words they lack, and the ISO list's function words ("across", "although", "everyone", "therefore", "whereas"), with every ISO content word left out. Every other language uses the `stop-words` ISO list. Logged in stage 4's landmines.
- Water counts a word on both the stop and filler lists ("just", "quite") once. The TS package counted it twice.
- Page charsets: Safi said yes to `encoding_rs` on 2026-11-13. Bodies are decoded the way browsers pick an encoding: byte order mark, `Content-Type` charset, a `<meta>` charset in the first 1024 bytes, then UTF-8, and windows-1252 when the bytes are not UTF-8. A body cut at the 10 MB cap inside a character still counts as UTF-8. HTML from Chrome (`--render-crawl`) is UTF-8 already and skips this. The `<meta>` check is one regex rather than the full WHATWG prescan. Stage 4 and the 06 stack table are updated.
- Snowball stems replace the TS plural-only lemma, for every language rust-stemmers covers. English keyword rows now group verb forms too ("composting" with "compost"). The row still shows the most written form.
- The declared `lang` wins over detection, as FR-TEXT-01 says, even when whatlang disagrees. On the 9-word fixture pages whatlang says `fr` at 0.23 confidence while the page declares `en`; the report shows both.
- Scripts without word spaces are decided by the text's script (whatlang), falling back to the language code when there is no text. Those pages get a character count only, and `content/keyword-stuffing` passes with "Word-based check not available for this language".
- New metrics beyond the TS package: `content/detected-language` and `content/avg-word-length` (FR-TEXT-01 and FR-TEXT-05's neutral fallback).
- Heading spans are byte offsets, as FR-TEXT-07 says. The TS package used UTF-16 offsets.
- Each text collector and the stuffing rule analyse the page again: 4 passes of about a millisecond each. A cache on the rule context is the upgrade if profiling ever shows it.
- `PageResult.language` has `#[serde(default)]`, so reports saved by 0.2.x still load from the history.

## M5: run history

Merged in PR #1 and released as 0.2.0 on 2026-11-10. Every CLI audit is saved to a local SQLite history, runs can be listed, shown again, compared and pruned, and `--baseline` adds a changes panel to both reports.

### Acceptance boxes

- [x] FR-HIST-01: two runs make two rows, a `--no-save` run none.
- [x] FR-HIST-02: `sitevet history` lists the fixture runs newest first.
- [x] FR-HIST-03: `sitevet show 1 --format json` is byte for byte what the audit printed.
- [x] FR-HIST-04: fixing one fixture rule between runs shows it under fixed, with the category score change.
- [x] FR-HIST-05: `prune --keep 2` leaves the newest 2, and deleting asks first or needs `--yes`.

### Decisions made in M5

- MSRV raised from 1.88 to 1.89: rusqlite 0.41, the stack's version, needs it. The spec caps the MSRV at 1.90.
- The `started_at` column holds the report's `generated_at` (when the audit finished). The report records no start time.
- 0.2.2: a new `PageRendered` event (url, ok, index, pages) per page the browser or PSI finished, added to spec stages 03 and 05 and synced into 06. The CLI shows a status line that rewrites itself through the crawl and the render, in a terminal only, so logs stay as they were.
- Link checks retry with GET when HEAD returns any error status, not only 405 or 501. Found on Safi's run 5 (2026-11-11): the VS Code marketplace and Figma answer HEAD with 404 for pages that load. This differs from the TS package, which has the same gap.
- 0.2.2: `links/broken-external` reports status 999 (LinkedIn turning away automated requests) as an unscored `info` "could not be checked" instead of a broken link. This differs from the TS rule.
- A diff compares a finding only when its rule has a result on that page in both runs. Otherwise the check ran on one side only (a `--browser` run against one without) and the finding is counted as "not compared" instead of fixed or new. Found on Safi's first 0.2.0 baseline run, fixed after the release.
- "Failing" in a diff means fail or warn. A finding whose evidence changed gets a new key, so it shows as one fixed and one new.
- Off a terminal, `history delete` and `prune` refuse without `--yes` rather than delete unasked.
- `[cli]` in `sitevet.toml` now also takes `baseline` and `save`.
- The fixture server can switch a site's routes between two runs, for the diff test.
- `cargo deny` allows MPL-2.0 for `option-ext`, which `directories` pulls in, as it already did for scraper's CSS crates.

## M4: rendered stage

The `browser` feature (off in the library by default, on in the CLI) renders every page in the installed Chrome or Chromium and runs axe-core 4.14.0 and Core Web Vitals. Without a browser, a PageSpeed Insights key does the same for the first `psi_max_pages` pages. With neither, the 7 rendered-stage rules are listed as skipped. All 94 rules can now run.

CI run 42866636468 passed every job, the new Chrome job included.

Tests: 3 Chrome tests in `tests/browser.rs`, 1 in `tests/browser_cancel.rs` and the guard test in `render_stage/browser.rs` (run with `SITEVET_TEST_CHROME=1`, and in a new Linux CI job with the runner's Chrome), the missing-Chrome test (always), the PSI test against a recorded response, the skipped-rules test, and a gzip test. Live: 6 pages of abdulkadersafi.com with `--browser` in 18 s, accessibility 100, LCP 1.04 s, CLS 0.005, no Chrome left running.

### Acceptance boxes

- [x] FR-RENDER-01: an unlabelled input gives a failing `accessibility/label`.
- [x] FR-RENDER-02: `performance/lcp` and `performance/cls` report numbers for a rendered fixture.
- [x] FR-RENDER-03: with a recorded PSI response the same rules fill in, and a 20-page audit sends 5 requests (plus one retry after a 429).
- [x] FR-RENDER-04: a default audit lists the browser rules as "needs browser or psi_key", and they do not affect the score.
- [x] FR-RENDER-05: `chrome_path` pointing nowhere fails before crawling, naming the option and how to install Chrome.
- [x] FR-RENDER-06: CI fails if the default library's tree contains chromiumoxide.
- [x] FR-API-03, the Chromium half: no Chrome process 1 s after cancelling mid-crawl.
- [x] FR-CRAWL-10 (Should): `render_crawl` finds a link added by JavaScript.

### Decisions made in M4

- Event order follows the TS package: `RenderComplete` comes before `CrawlComplete`, because the check total in `CrawlComplete` depends on which pages rendered. Stage 5's event table lists them the other way round.
- axe runs its default rule set, as the TS package's AxeBuilder did, not only the WCAG tags the M0 spike used.
- Fixed a bug that the live run exposed: reqwest's decoder deletes `Content-Encoding`, so `performance/text-compression` warned on every page. sitevet now turns off reqwest's automatic decoding for the crawl, sends `Accept-Encoding` itself and decodes with `async-compression` (already in the tree through reqwest's tower-http). That adds one direct dependency the stack table does not list, and keeps the 10 MB cap applied to decoded bytes.
- Two TS quirks fixed rather than ported: PSI pass IDs were prefixed twice (`accessibility/accessibility/...`), and PSI failures had the fix text "See " with no link. sitevet keeps the rule's own fix text when there is no link.
- The PSI fixture is in the real v5 response shape but hand-trimmed, since no key was available to record a live response.
- `SITEVET_PSI_ENDPOINT` (not in stage 5's list) points the library at a recorded PSI response in tests. Setting an environment variable is `unsafe` in edition 2024, so the workspace lint went from `forbid` to `deny(unsafe_code)`, with one documented `allow` in `tests/psi.rs`.
- The cancel test lives in its own test binary: it counts Chrome processes by a profile name that carries the process ID, and other tests launching Chrome in the same process would be counted too.
- Security fix from an automated review: when the start URL is public, Chrome's own requests (navigations, redirects, scripts, images) now obey the same private-network rule as the fetcher. Each tab intercepts its requests through CDP's Fetch domain and fails those to private, loopback, link-local or metadata addresses. Chrome resolves names again after the check, so DNS rebinding between the two is not covered for browser requests; the fetcher's own requests are. A Chrome test checks that a page's image and `fetch()` reach a local listener with the guard off and never with it on.
- Chrome is found from `chrome_path`, `SITEVET_CHROME`, `PATH` and the usual install folders. Microsoft Edge is not used, since the spec names Chrome and Chromium.

### Open items for Safi

- From Safi's first real runs of 0.1.0 (2026-11-10): crawl and render progress, and LinkedIn's status 999. Both done for 0.2.2 (see below).
- The overall score formula (from M3) is still open.

## M3: reports and CLI

Reports: JSON (pretty, `schema_version` 1, with a generated JSON Schema committed as `schema/report.schema.json`), Markdown and a self-contained HTML file, all from askama templates in `crates/sitevet/templates/`. CLI: `sitevet audit`, `rules` and `init`. Release: cargo-dist 0.34 configured; nothing published yet. CI run 42833300105 passed every job.

Tests: 79 unit tests, 28 library integration tests (7 new in `tests/report.rs`), 12 CLI tests (8 in `crates/sitevet-cli/tests/cli.rs`), 2 doc tests. Snapshots with insta: the Markdown and HTML reports of the basic fixture site, CLI stderr, and the `rules` table.

Live check on abdulkadersafi.com (15 pages, 8 s): overall 86, exit 0. Its HTML report scored 100 for accessibility in Lighthouse (axe-core) in dark mode on desktop and in light mode at 360 px.

### Acceptance boxes

- [x] FR-API-04 (from M1): every field through TOML and through flags gives equal Options.
- [x] FR-API-05: each helper equals `render` on `audit`'s report.
- [x] FR-RULE-02 (from M2): every finding from four fixture audits validates against the schema.
- [x] FR-REPORT-01: every fixture report validates, and the committed schema equals the generated one.
- [x] FR-REPORT-02: Markdown snapshot passes. Rendering on GitHub is part of Safi's review.
- [x] FR-REPORT-03: no external requests (checked in a test), axe-core via Lighthouse at 100. The axe check moves into the test suite when the browser stage lands in M4.
- [x] FR-REPORT-04: the Arabic fixture's title is quoted in a `dir="auto"` element.
- [x] FR-CLI-01, FR-CLI-02, FR-CLI-03, FR-CLI-04, FR-CLI-05, FR-CLI-06.
- [x] FR-CLI-07 (Should): `sitevet init`.
- [x] FR-DIST-01: `cargo install sitevet-cli` from crates.io works, and a fresh project depending on `sitevet` has no clap, rusqlite or chromiumoxide in its tree.
- [x] FR-DIST-02: the `install-check` workflow ran the released installers on fresh Ubuntu, macOS and Windows runners (run 42872388546).

### Decisions made in M3

- The report's format follows `--out`'s extension when `--format` is not given, else HTML (the TS package's default).
- Status words for scores, shared by the HTML badge and the CLI: 90 and up PASS ("Good"), 50 and up WARN ("Needs work"), below FAIL ("Poor").
- `--fail-on error` trips on any failing error-severity finding. `--fail-on warning` trips on any failing or warning finding of error or warning severity.
- `--quiet` prints only errors and the threshold line. `--plain` turns colour off; anstream already honours `NO_COLOR` and `CLICOLOR`.
- The HTML report lists the findings that need attention and counts passes per category, and keeps measurements in a collapsed section, so a 100-page report stays readable.
- A link robots.txt keeps sitevet from requesting is no longer reported as a broken link (it showed as "no response" before).
- `tracing-subscriber` is added to the CLI for `--verbose` and `SITEVET_LOG`. Stage 5 says the CLI installs a subscriber but the stack table does not name the crate.
- `cargo deny` ignores RUSTSEC-2026-0374: `basic-toml` is unmaintained, and askama's derive macro uses it at compile time only.
- The delay test allows 100 ms of server-side arrival jitter; the Windows runner delivered a fresh connection's first request 55 ms late.
- `Options::KEYS` and a hidden `unknown_key_reason` are public so the CLI can suggest keys for its `[cli]` table too.

### Open items for Safi

- Review: the HTML report (`sitevet audit <url> --out report.html`), the CLI output, and the README.
- Release 0.1.0: held during M4, then released on 2026-11-10 at Safi's request, so it includes M4. Both crates are on crates.io and the `v0.1.0` GitHub Release has the binaries and installers.
- The overall score uses the TS package's harsh formula. On the bare fixture site it is 0 while every page scores 64 to 77, because many rules fail on every page. That is parity, but it reads oddly in a report. Keep it?

## M2: parity rules, metrics, scoring

All 94 rules from the TS package at `de27aea`, in `crates/sitevet/src/rules/<category>.rs`, one unit struct and one registry line each. 87 run today (85 in the Dom stage, 2 link rules in the Enriched stage). The 7 rendered-stage rules (`accessibility/axe` and 6 Core Web Vitals rules) have their logic and tests now and start running in M4. 14 collectors in `crates/sitevet/src/metrics/`.

CI run 42827541672 passed every job. Two earlier M2 runs failed on Windows only, both in tests: CRLF checkouts broke the CONTRIBUTING.md quote check, and server-side arrival times jittered past the delay test's margin. Both tests were fixed; the code under test did not change.

Tests: 77 unit tests (every rule and collector has a passing and a failing case, NFR-11), 21 integration tests, 2 doc tests. The TS package's own `rules.test.ts`, `metrics.test.ts` and `runner.test.ts` assertions are ported against the same 8 fixtures and give the same results and the same scores.

### Acceptance boxes

- [x] FR-RULE-01: 94 rules, same IDs, severities, categories and conditions. Every TS fixture expectation matches.
- [ ] FR-RULE-02: needs the report JSON Schema, which M3 builds for FR-REPORT-01. Every finding already carries all the fields. Ticked in M3.
- [x] FR-RULE-03: 3 axe violations give 3 findings with distinct IDs (synthetic rendered data; the real browser arrives in M4).
- [x] FR-RULE-04: `only = ["security"]` leaves only security rules, collectors and categories.
- [x] FR-MET-01: every phase 1 collector is ported with fixture tests, and scores are identical with collectors on and off. The phase 2 text collectors come with M6 (see decisions).
- [x] FR-MET-02: 300 links give 25 rows and a total of 300.
- [x] FR-SCORE-01: the TS scoring cases give identical numbers.
- [x] FR-SCORE-02: collectors and info findings leave every score unchanged. Skipped rules arrive with the browser stage in M4 and are checked again there.
- [x] FR-EXT-01: `examples/custom-rule.rs`; custom rules passed through `Options.rules` show up in the report in the engine tests.
- [x] FR-EXT-02: `CONTRIBUTING.md` quotes a real rule, its registry line and its test, and a test fails if the quote drifts from the source.

### Decisions made in M2

- `RuleCx` has two fields beyond stage 5's list: `html` (the raw markup, Dom stage only) and `rendered` (`RenderedData`, Rendered stage only). 13 of the 94 TS rules read the raw HTML, so a parity port needs it. Both are additive.
- Rules use the DOM the way the TS package's cheerio calls do: `body.text()` includes script text, `.attr()` reads the first match, and string lengths count UTF-16 units like JavaScript's `.length`. Link text is trimmed but not collapsed, and links keep non-http URLs (`mailto:`), as the TS `absUrl` does.
- Repeated response headers are joined with ", ". Node's header object kept one `Set-Cookie` value; joining checks every cookie, so `security/cookie-secure` can flag a cookie the TS package missed.
- `eeat/dates` treats a date as valid when it is ISO 8601 or RFC 2822 (HTTP header style). JavaScript's `Date.parse` also reads looser forms such as "January 15, 2026", which show as `warn` here.
- `content/keyword-stuffing` reads a new `text/` module (main content, unicode-segmentation words and sentences, English stop words, the plural lemma, n-grams), ported from the TS `src/text.ts`. unicode-segmentation is the stack's M6 tokenizer, taken in early. M6 extends the module (FR-TEXT-08 asks for the rule on its n-gram output, which it already is).
- The phase 2 text collectors (`content/text-statistics`, `content/readability`, `content/keywords` and their values) are FR-TEXT-04 to FR-TEXT-07, which M6 builds. They are not in M2.
- Metric text follows sitevet's writing rules: "-" instead of an em dash for empty cells and the heading outline indent, and "..." instead of the ellipsis character. The TS package's own metric tests are ported with that one substitution.
- Selectors and regexes are compiled per call. Rule CPU time is far under NFR-01's 50 ms per page at today's sizes; caching is the upgrade if it ever is not.
- `regex` (M11's stack crate) is used now, since the TS rules are regex based.

### Parity drift

At the end of M2 the TS repo's HEAD is still `de27aea`, with a clean working tree and `origin/main` at the same commit. No differences to log.

### Open items for Safi

- FR-RULE-02's acceptance test needs the JSON Schema that M3 generates. Fine to tick it in M3, or should the schema move into M2?
- Phase 2 text collectors sit in M6 with the text engine. Done in M6.
- Character sets (from M1): `encoding_rs` added in M6.

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

- Character sets: closed in M6. `encoding_rs` decodes page bodies.

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
