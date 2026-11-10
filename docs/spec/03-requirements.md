# Functional requirements

Status: signed off 2026-11-10

Requirements say what the product must do, not how. Tech choices are in Stage 4. Each traces to the product (Stage 1) and the journeys or core loop (Stage 2).

"The library" means the `sitevet` crate's public API. "The CLI" means the `sitevet` binary. Unless a requirement says CLI only, the library must expose the capability and the CLI must reach it through a flag or a `sitevet.toml` key. "The TS package" is the TypeScript predecessor, used as the reference for parity behaviour.

## Core features (MVP)

### Audit entry points

- **FR-API-01 [Must]** The library must expose an async `audit(url, options)` that crawls, runs every enabled rule and collector, scores the result, and returns a typed `AuditReport`.
  _Acceptance:_ given a local fixture server with 3 linked pages, `audit` returns a report with 3 pages, a score from 0 to 100, findings and metrics per page.
- **FR-API-02 [Must]** The library must expose `audit_stream(url, options)` that returns a stream of typed progress events, ending with an event that carries the final report.
  _Acceptance:_ the stream yields `AuditStart`, then at least one `PageFetched`, then `CrawlComplete`, then `CheckComplete` and `PageComplete` events, then `AuditComplete` with a report equal to what `audit` returns for the same input.
- **FR-API-03 [Must]** Dropping the audit future or stream, or calling cancel on it, must stop all network and browser work.
  _Acceptance:_ cancelling a 100-page audit mid-crawl leaves no open connections and no Chromium process after 1 second.
- **FR-API-04 [Must]** All options must live in one public `Options` struct with documented defaults. The CLI flags and the `sitevet.toml` keys map one to one onto its fields.
  _Acceptance:_ a test sets every field through TOML and through flags and gets equal `Options` values.
- **FR-API-05 [Must]** Convenience functions must render a report or return just the score: `render(&report, format)`, plus one-call helpers equivalent to the TS package's `auditToHtml`, `auditToMarkdown` and `auditScore`.
  _Acceptance:_ each helper's output equals `render` on the report from `audit` for the same site.
- **FR-API-06 [Must]** The library must run inside an app that does not use tokio as its main executor (for example a GPUI app), without the caller managing a runtime by hand.
  _Acceptance:_ an example that calls `audit` from a non-tokio executor completes a fixture audit. The documented pattern is in the crate docs.

### Crawl

- **FR-CRAWL-01 [Must]** The crawler must follow same-origin links from the start URL, breadth first, up to `max_pages` (default 100) and `max_depth` (default 5), fetching `concurrency` pages at once (default 5).
  _Acceptance:_ on a fixture site with 150 linked pages, defaults crawl exactly 100 pages, none deeper than 5 clicks.
- **FR-CRAWL-02 [Must]** `max_pages` must be capped at 10,000. Higher values are rejected with a clear error.
  _Acceptance:_ `max_pages = 10001` fails validation with a message naming the cap.
- **FR-CRAWL-03 [Must]** `robots.txt` and `sitemap.xml` (including sitemap indexes and sitemaps listed in robots.txt) must be fetched once per site and shared with every rule.
  _Acceptance:_ a fixture server logs exactly one request each for `/robots.txt` and `/sitemap.xml` during a 20-page audit.
- **FR-CRAWL-04 [Must]** The crawler must obey `robots.txt` Disallow rules for its own user agent and for `*` by default. Skipped URLs appear in the report as "blocked by robots.txt". An `ignore_robots` option turns this off.
  _Acceptance:_ a disallowed path is not requested by default, is listed as blocked, and is requested when `ignore_robots = true`.
- **FR-CRAWL-05 [Must]** The crawler must seed the queue from the sitemap as well as from links (phase 4), controlled by a `crawl_sitemap` option (default on).
  _Acceptance:_ a page listed only in the sitemap and linked from nowhere is crawled.
- **FR-CRAWL-06 [Must]** Per-request timeout (default 15 s), custom user agent, and an optional per-host delay between requests (default 0 ms) must be configurable.
  _Acceptance:_ with `delay_ms = 500`, request start times to one host are at least 500 ms apart.
- **FR-CRAWL-07 [Must]** The default user agent must identify sitevet and its version and include the repository URL.
  _Acceptance:_ the fixture server sees `sitevet/<version> (+https://github.com/Abdulkader-Safi/sitevet)`.
- **FR-CRAWL-08 [Must]** Redirects must be followed and recorded (chain length and final URL). URL normalisation must remove fragments and treat `/a` and `/a/` according to the server's redirect, so a page is never audited twice.
  _Acceptance:_ `/a`, `/a/` and `/a#x` on a fixture site produce one audited page.
- **FR-CRAWL-09 [Must]** A shared link checker must check each distinct URL once per audit, whichever rules ask about it.
  _Acceptance:_ a URL linked from 10 pages is requested once.
- **FR-CRAWL-10 [Should]** With the browser feature on, a `render_crawl` option must fetch each page through Chromium, so links and content added by JavaScript are discovered and audited.
  _Acceptance:_ on a fixture SPA whose links exist only after JS runs, `render_crawl = true` finds them and the default crawl does not.
- **FR-CRAWL-11 [Must]** If the start URL cannot be fetched (DNS failure, connection refused, non-HTML response), the audit must fail with a typed error. Errors on other pages become findings on that page and the audit continues.
  _Acceptance:_ an unreachable start URL returns an error; one 500 page among 10 produces a report with 10 pages, one marked with its status.

### Rule catalog (parity with the TS package)

- **FR-RULE-01 [Must]** All 94 rules in appendix A must be ported with the same rule IDs and severities as the TS package (and the same category for each), and the same conditions for each status.
  _Acceptance:_ each rule has a fixture test in which it passes on a good fixture and fails on a bad one. The TS package's fixtures (`test/fixtures/*.html`) are ported and every expected result matches.
- **FR-RULE-02 [Must]** Every finding must carry rule ID, category, title, status (pass, fail, warn, info), severity (error, warning, info), message, and optional details, evidence (the offending markup, header or value) and fix text.
  _Acceptance:_ the JSON schema test validates every finding from the fixture suite.
- **FR-RULE-03 [Must]** One rule may emit many findings, each with its own ID override and severity (as the accessibility rule does for each axe violation).
  _Acceptance:_ a fixture with 3 axe violations yields 3 accessibility findings with distinct IDs.
- **FR-RULE-04 [Must]** Categories must be filterable with `only` and `skip` lists.
  _Acceptance:_ `only = ["security"]` runs only security rules and collectors, and the report has no other categories.

### Measurements (parity with the TS package)

- **FR-MET-01 [Must]** Every collector in the TS package (`src/metrics/*.ts`, IDs in appendix B) must be ported. Metrics carry a value or a table plus a status of ok, warn, bad or missing, and never change the score.
  _Acceptance:_ each collector has a fixture test. A test confirms the score is identical with collectors on and off.
- **FR-MET-02 [Must]** Table metrics must be capped at a row limit with a "showing N of M" total.
  _Acceptance:_ a fixture with 300 links gives a link table of at most the cap and a total of 300.

### Rendered stage (browser and PageSpeed)

- **FR-RENDER-01 [Must]** With the browser option on, each page must be rendered in headless Chrome or Chromium found on the machine, and an axe-core accessibility pass must run, one finding per violation and one pass per passed check.
  _Acceptance:_ on a fixture with a missing form label, an `accessibility/*` failure for `label` appears.
- **FR-RENDER-02 [Must]** The rendered stage must collect Core Web Vitals and page stats: LCP, CLS, TTFB, DOM node count, page weight, request count.
  _Acceptance:_ the `performance/lcp` and `performance/cls` rules report numbers for a rendered fixture.
- **FR-RENDER-03 [Must]** Without a local browser, a `psi_key` option must get Core Web Vitals and accessibility from the Google PageSpeed Insights API for the first `psi_max_pages` pages (default 5).
  _Acceptance:_ with a recorded PSI response, the same rules fill in, and only 5 requests are made for a 20-page audit.
- **FR-RENDER-04 [Must]** Rules that need the rendered stage must be skipped, and listed as skipped with a reason, when neither browser nor PSI is enabled.
  _Acceptance:_ a default audit lists the browser rules as "skipped: needs browser or psi_key", and they do not affect the score.
- **FR-RENDER-05 [Must]** If browser is requested but no Chrome binary is found, the audit must fail before crawling, with a message saying how to install one or set its path.
  _Acceptance:_ with `chrome_path` pointing nowhere, the error names the option and the install hint.
- **FR-RENDER-06 [Must]** The browser stage must be a cargo feature. Building without it pulls in no browser dependencies.
  _Acceptance:_ `cargo tree --no-default-features` shows no browser crates.

### Text engine (phase 2)

- **FR-TEXT-01 [Must]** Detect the page language from `<html lang>`, falling back to detection from the body text, and report both values.
  _Acceptance:_ an Arabic fixture with no `lang` attribute is detected as Arabic.
- **FR-TEXT-02 [Must]** Main-content extraction must exclude boilerplate (nav, header, footer, aside) from text statistics.
  _Acceptance:_ a fixture whose footer repeats a word 50 times does not show that word in the top keywords.
- **FR-TEXT-03 [Must]** Unicode-aware tokenizing, stop words and stemming for the page language, for every language the chosen libraries support. English and Arabic are the tested minimum. Languages without a stop word list still get counts, with stop-word figures marked unavailable.
  _Acceptance:_ English and Arabic fixtures produce correct word counts and stop-word percentages against hand-counted values.
- **FR-TEXT-04 [Must]** Text statistics: total words, unique words and percent, stop words and percent, water percent, average sentence length, characters without spaces.
  _Acceptance:_ fixture values match hand-computed values within 1%.
- **FR-TEXT-05 [Must]** Readability: Flesch reading ease with a label, Flesch-Kincaid grade and Gunning Fog for English. For other languages, use a published formula where one exists for that language, otherwise report average sentence and word length and mark the formula "not available for this language".
  _Acceptance:_ the English fixture matches reference values. An Arabic fixture shows the "not available" state, never an English score.
- **FR-TEXT-06 [Must]** Top phrases as 1, 2 and 3-word n-grams with counts and percentages, plus a stop word table.
  _Acceptance:_ a fixture with a known repeated phrase lists it first among 2-word phrases.
- **FR-TEXT-07 [Must]** Keyword relevance matrix: each of the top 10 keywords present or absent in title, meta description and H1. Plus highlight spans for those keywords in every heading.
  _Acceptance:_ spans in the fixture point at the exact byte ranges of the keyword.
- **FR-TEXT-08 [Must]** `content/keyword-stuffing` must use the n-gram output.
  _Acceptance:_ a stuffed fixture fails, a normal one passes.

### Answer engine and content structure (phase 3)

- **FR-AEO-01 [Must]** Answer snippet check: a definition or short answer within the first four content blocks, which block, and its length in words.
  _Acceptance:_ a fixture with a 40-word definition in block 2 reports block 2, 40 words.
- **FR-AEO-02 [Must]** Q&A structure: count of question-like H2s, how many have a short answer directly under them, and the coverage ratio.
  _Acceptance:_ a fixture with 4 question H2s, 3 answered, reports 0.75.
- **FR-AEO-03 [Must]** Answer and passage formats: counts of lists, steps, tables, definition blocks and FAQ blocks. H2 section count with the average words per section, plus the shortest and longest, against a target band.
  _Acceptance:_ fixture counts match.
- **FR-AEO-04 [Must]** Topic cluster signals: internal links inside main content only (total and unique), related-reading section detection, distinct H2 count next to H3 count, and the anchor descriptiveness ratio.
  _Acceptance:_ nav links are excluded from the main-content link count on the fixture.
- **FR-AEO-05 [Must]** Agent experience: whether robots.txt allows each AI crawler (GPTBot, ClaudeBot, PerplexityBot, CCBot, Google-Extended). Whether `llms.txt` exists and parses. Whether the main content is present in the raw HTML without JavaScript.
  _Acceptance:_ a robots fixture blocking GPTBot reports GPTBot blocked and the others allowed.
- **FR-AEO-06 [Must]** Schema depth: a nested type tree of all structured data with a presence status per node, read from JSON-LD, microdata and RDFa. FAQPage and HowTo validation when present.
  _Acceptance:_ a microdata fixture produces the same tree as its JSON-LD twin.
- **FR-AEO-07 [Should]** Page-type hint: the page purpose (article, product, local business, home, contact) inferred from schema and structure.
  _Acceptance:_ the article fixture is labelled article.

### Site-wide intelligence (phase 4)

- **FR-SITE-01 [Must]** Duplicate titles, meta descriptions and H1s across the crawl, each grouped with the URLs that share it.
  _Acceptance:_ a fixture site with 3 pages sharing one title reports one group of 3.
- **FR-SITE-02 [Must]** Near-duplicate content detection across pages through a shingle hash, with a similarity percent per pair above a threshold.
  _Acceptance:_ two fixture pages differing by one paragraph are flagged. Two unrelated pages are not.
- **FR-SITE-03 [Must]** Orphan pages: in the sitemap but not linked from any crawled page.
  _Acceptance:_ the sitemap-only fixture page is reported as orphaned.
- **FR-SITE-04 [Must]** Internal link graph with inbound link count per URL, and click depth per page with a warning past a threshold (default 3).
  _Acceptance:_ fixture counts match the known graph.
- **FR-SITE-05 [Must]** Site-wide redirect map: every internal link that lands on a redirect and where it ends.
  _Acceptance:_ a link to `/old` that redirects to `/new` appears with both.
- **FR-SITE-06 [Must]** Sitemap versus crawl reconciliation in both directions: in the sitemap but not crawlable, and crawlable but not in the sitemap.
  _Acceptance:_ the fixture site produces one entry in each list.

### Trust and compliance (phase 5)

- **FR-TRUST-01 [Must]** Author box or byline detected as a unit, author page link, short author bio, and the actual publish and updated dates as values.
  _Acceptance:_ the article fixture reports author name, link, bio present and both dates.
- **FR-TRUST-02 [Must]** Structured data trust signals listed by name: which of these are present: Article, BlogPosting, Organization, author, datePublished, dateModified.
  _Acceptance:_ fixture table matches.
- **FR-TRUST-03 [Must]** Trust pages and details: refund or returns page, phone number and physical address, ads or affiliate disclosure, editorial policy page. Detection uses link text and URL patterns in English plus common equivalents in the page language where the libraries allow.
  _Acceptance:_ an English fixture with a `/refund-policy` link passes the refund check.
- **FR-TRUST-04 [Should]** Citation quality: share of outbound citations pointing at a bundled list of recognised authoritative domains (government, education, major standards bodies, encyclopedias).
  _Acceptance:_ a fixture citing `who.int` and a random blog reports 50%.
- **FR-TRUST-05 [Must]** Local SEO: LocalBusiness schema completeness (address, geo, opening hours), NAP consistency between page text and schema, and a Google Business Profile link.
  _Acceptance:_ a fixture whose page phone differs from its schema phone reports a NAP mismatch.

### Performance depth (phase 6)

- **FR-PERF-01 [Must]** With the rendered stage, report total load time, DNS, TCP, response time, DOM interactive, DOM content loaded, DOM complete and FCP.
  _Acceptance:_ all values are present and non-negative for a rendered fixture.
- **FR-PERF-02 [Must]** INP reported with an explicit "not measurable without interaction" state when no interaction happened, never as zero or empty.
  _Acceptance:_ a default rendered audit shows that state.
- **FR-PERF-03 [Must]** Layout shift event count and largest single shift. Slow resource list: every request over a threshold (default 300 ms), or a clean "none exceeded" result.
  _Acceptance:_ a fixture with a delayed image lists it.
- **FR-PERF-04 [Must]** Static performance checks that need no browser: font loading (`font-display`, preloads, font file count), HTTP/2 or HTTP/3 in use, resource hints (preconnect, dns-prefetch) for third-party origins, third-party script count and weight.
  _Acceptance:_ each check passes and fails on matching fixtures.

### Tech stack fingerprinting (phase 7)

- **FR-TECH-01 [Must]** Detect, per vendor, analytics tools (Google Analytics, Google Tag Manager, Hotjar, Clarity, Matomo, Plausible, Fathom) and ad pixels (Google Ads, Meta, TikTok, LinkedIn, X, Pinterest, Snap), replacing the single "tag present" result with a per-vendor table.
  _Acceptance:_ a fixture with GA4 and the Meta pixel lists exactly those two.
- **FR-TECH-02 [Must]** Detect CMS (WordPress, Shopify, Webflow, Wix, Drupal, Joomla, Squarespace, Ghost), frameworks and libraries (React, Next.js, Vue, Nuxt, Angular, Svelte, jQuery, Laravel), and CDN (Cloudflare, Fastly, Akamai, CloudFront, Vercel, Netlify) from markup, script sources and headers.
  _Acceptance:_ recorded responses from a WordPress-on-Cloudflare fixture detect both.
- **FR-TECH-03 [Must]** Signatures must live in a data file, so adding a vendor is one entry with no engine change.
  _Acceptance:_ a test adds a signature in test data and detects it.

### Media and assets (phase 8, opt-in)

- **FR-MEDIA-01 [Must]** With a `media` option on (default off), fetch each image with HEAD or a ranged GET, capped by `media_max_requests` (default 200), and report a per-image table: URL, pixel dimensions, transfer size, format, optimisation verdict.
  _Acceptance:_ with the option off, no image requests are made. With it on, the fixture table lists dimensions and sizes.
- **FR-MEDIA-02 [Should]** Per-image optimisation estimate (for example "about 15 to 25% smaller as WebP") from a per-format heuristic.
  _Acceptance:_ a large PNG fixture gets an estimate.
- **FR-MEDIA-03 [Must]** Responsive image checks (`srcset` and `sizes` where they matter), video schema, thumbnails and captions, and favicon and app icon presence and sizes.
  _Acceptance:_ each check passes and fails on matching fixtures.
- **FR-MEDIA-04 [Must]** With the rendered stage, images served much larger than their layout box (default over 2x in both dimensions).
  _Acceptance:_ a 2000 px image shown at 300 px is flagged.

### Mobile and rendered extras (phase 9)

- **FR-MOBILE-01 [Must]** With the rendered stage at a mobile viewport: tap targets below the minimum size, text too small to read, horizontal overflow, intrusive interstitials.
  _Acceptance:_ each is flagged on a matching fixture.
- **FR-MOBILE-02 [Must]** Viewport behaviour without a browser: zoom disabled, fixed widths.
  _Acceptance:_ `user-scalable=no` is flagged.
- **FR-MOBILE-03 [Must]** Console errors captured during load, and the content that disappears when JavaScript is off (rendered text compared with raw text).
  _Acceptance:_ a fixture that throws on load reports the error. An SPA fixture reports the missing content share.
- **FR-MOBILE-04 [Must]** Screenshots, full page and above the fold, at desktop and mobile widths, saved as files next to the report. Off by default (`screenshots` option). The HTML report embeds above-the-fold thumbnails as base64 and links the full-page files.
  _Acceptance:_ with the option on, PNG files exist, the HTML report shows the thumbnails offline, and the full-page links open the files.

### Scoring

- **FR-SCORE-01 [Must]** A health score from 0 to 100 per page, per category and overall, weighted by severity, using the same formula as the TS package so the same findings give the same score.
  _Acceptance:_ the ported TS scoring tests produce identical numbers.
- **FR-SCORE-02 [Must]** Skipped rules, info findings and measurements never change the score.
  _Acceptance:_ toggling collectors and skipped rules leaves the score unchanged.

### Report output

- **FR-REPORT-01 [Must]** JSON output: the full report with a `schema_version` field, documented in a published JSON Schema file.
  _Acceptance:_ every fixture report validates against the schema.
- **FR-REPORT-02 [Must]** Markdown output: score summary and per-category headings, each with its findings and metric tables.
  _Acceptance:_ a snapshot test of the fixture report passes, and the output renders on GitHub.
- **FR-REPORT-03 [Must]** HTML output: one self-contained file with inline CSS and no external requests, an overall score badge, per-category sections, colour-coded findings with text labels, metric tables with "showing N of M", and the site-wide sections. Neutral sitevet styling, no company branding.
  _Acceptance:_ the file opens offline with zero network requests and passes an axe check at WCAG 2.2 AA.
- **FR-REPORT-04 [Must]** Text quoted from a right-to-left page must display in its own direction without breaking the layout.
  _Acceptance:_ an Arabic fixture's title shows right to left in the HTML report.

### Progress events

- **FR-EVENT-01 [Must]** Events: `AuditStart`, `PageFetched`, `CrawlComplete`, `RenderComplete`, `CheckComplete`, `PageComplete`, `AuditComplete`, with the same fields as the TS package events. Events after `CrawlComplete` carry progress against a known total.
  _Acceptance:_ a fixture audit's event log matches the expected sequence and counts.
- **FR-EVENT-02 [Must]** Events must be `Send` and serialisable, so an app can forward them across threads or over a channel.
  _Acceptance:_ a compile test sends events through a `std::sync::mpsc` channel and serialises one to JSON.

### CLI

- **FR-CLI-01 [Must]** `sitevet audit <url>` runs an audit with flags for every `Options` field, plus `--format json|md|html`, `--out <path>` and `--config <path>`. Without `--out`, the report goes to stdout. Progress and logs go to stderr.
  _Acceptance:_ `sitevet audit URL --format json > r.json` produces valid JSON, with nothing else in the file.
- **FR-CLI-02 [Must]** Read `sitevet.toml` from the current folder or `--config`. Flags override file values. Unknown keys are an error that names the key.
  _Acceptance:_ a misspelled key fails with its name and the closest valid key.
- **FR-CLI-03 [Must]** Default progress shows one line per page (`3/100  https://site/about  92/100`) and a summary with the overall score and the counts of failures per category.
  _Acceptance:_ snapshot test of stderr on a fixture site.
- **FR-CLI-04 [Must]** `--fail-under <score>` and `--fail-on error|warning` for CI. Exit codes: 0 when the audit ran and passed thresholds, 1 when a threshold failed, 2 when the audit could not run (bad URL, unreachable start page, bad config, missing browser).
  _Acceptance:_ each exit code is produced by a matching test case.
- **FR-CLI-05 [Must]** Output respects `NO_COLOR`, prints status words (PASS, FAIL, WARN) next to colours, and has `--quiet` and `--plain` modes (no animation, no colour).
  _Acceptance:_ with `NO_COLOR=1`, stderr contains no ANSI escape codes.
- **FR-CLI-06 [Must]** `sitevet rules` lists every rule with its ID, category and severity, and whether it needs the browser.
  _Acceptance:_ the list has the same count as the registry.
- **FR-CLI-07 [Should]** `sitevet init` writes a commented `sitevet.toml` with every key at its default.
  _Acceptance:_ the written file parses back into default `Options`.

### Run history

- **FR-HIST-01 [Must]** Every CLI audit is saved to a local SQLite database in the OS data folder, unless `--no-save` is passed. The library offers the same store behind a `history` cargo feature, off by default.
  _Acceptance:_ two runs create two rows. A run with `--no-save` creates none.
- **FR-HIST-02 [Must]** `sitevet history [url]` lists past runs with ID, date, URL, page count and overall score, newest first.
  _Acceptance:_ output lists the fixture runs in order.
- **FR-HIST-03 [Must]** `sitevet show <id> [--format]` renders a stored run without crawling again.
  _Acceptance:_ the output equals the report rendered at audit time.
- **FR-HIST-04 [Must]** `sitevet diff <id-a> <id-b>` (and `--baseline <id|file>` on `audit`) shows score change per category, new failures and fixed failures. The HTML and Markdown reports gain a "changes since last run" section when a baseline is given.
  _Acceptance:_ fixing one fixture rule between runs shows it under fixed, with the category score change.
- **FR-HIST-05 [Must]** `sitevet history delete <id>` and `sitevet history prune --keep <n>` remove runs, asking for confirmation unless `--yes` is passed.
  _Acceptance:_ prune with `--keep 2` leaves the newest 2 runs.

### Extending

- **FR-EXT-01 [Must]** Public `Rule` and `Collector` traits. A caller can pass a custom list that replaces or extends the built-in registry.
  _Acceptance:_ an example crate adds one custom rule and sees its findings in the report.
- **FR-EXT-02 [Must]** A built-in rule is one source file plus one registry line, with a fixture test pattern contributors can copy. A `CONTRIBUTING.md` explains it.
  _Acceptance:_ the contributing guide's worked example compiles and passes.

### Distribution

- **FR-DIST-01 [Must]** Publish the crate to crates.io and install the CLI with `cargo install sitevet`.
  _Acceptance:_ a clean machine installs and runs `sitevet --version`.
- **FR-DIST-02 [Must]** Prebuilt CLI binaries on GitHub Releases for macOS arm64 and x86_64, Linux x86_64 and arm64 (static), Windows x86_64, plus a one-line shell installer.
  _Acceptance:_ the installer works on a fresh macOS and a fresh Ubuntu machine.
- **FR-DIST-03 [Should]** A Homebrew tap.
  _Acceptance:_ `brew install` installs a working binary.

## Standard states

| Item                               | Coverage                                                                                                                                |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Undo and redo                      | Out of scope: no editing. Runs are read-only snapshots.                                                                                 |
| Empty states                       | FR-CLI-03 and FR-HIST-02: "no runs yet" and "0 pages crawled" messages. FR-RENDER-04 lists skipped rules.                               |
| Loading and progress               | FR-API-02, FR-EVENT-01, FR-CLI-03.                                                                                                      |
| Error states and messages          | FR-CRAWL-11, FR-RENDER-05, FR-CLI-02, FR-CLI-04.                                                                                        |
| Confirm before destructive actions | FR-HIST-05.                                                                                                                             |
| Autosave or save and reopen        | FR-HIST-01 and FR-HIST-03.                                                                                                              |
| Search and filter                  | FR-RULE-04 (categories), FR-HIST-02 (by URL).                                                                                           |
| Keyboard shortcuts                 | Out of scope: non-interactive CLI.                                                                                                      |
| Settings that persist              | FR-CLI-02 (`sitevet.toml`).                                                                                                             |
| First run and onboarding           | FR-CLI-07 (`sitevet init`), README quick start, crate docs example.                                                                     |
| Offline or bad connection          | FR-CRAWL-06 timeouts, FR-CRAWL-11 per-page errors. No offline mode: an audit needs the site. `show`, `history` and `diff` work offline. |
| Permissions per role               | Out of scope: single-user tool.                                                                                                         |
| Notifications                      | Out of scope: exit codes and stderr only.                                                                                               |
| Import and export                  | FR-REPORT-01 to 03 export. FR-HIST-04 imports a JSON file as a baseline.                                                                |
| Delete and restore                 | FR-HIST-05 delete. No restore: deleted runs are gone.                                                                                   |
| Audit log                          | Out of scope: the run history is the record.                                                                                            |
| Multi-language and RTL             | Tool text English only (stage 2). FR-TEXT-01, FR-TEXT-03, FR-REPORT-04 handle page content in any language.                             |
| Accessibility                      | FR-REPORT-03 (WCAG 2.2 AA HTML), FR-CLI-05.                                                                                             |
| Mobile layout                      | The HTML report must be readable at 360 px width (part of FR-REPORT-03 acceptance).                                                     |
| Rate limits and quotas             | FR-CRAWL-01 concurrency, FR-CRAWL-06 delay, FR-RENDER-03 PSI cap, FR-MEDIA-01 request cap.                                              |

## Account and auth

No accounts. sitevet audits public URLs only in v1. No basic auth, no custom headers, no cookies (chosen in stage 3 questions). The one credential is an optional PageSpeed Insights API key, read from the `psi_key` option or the `SITEVET_PSI_KEY` environment variable.

## Data the product handles

- Options: audit configuration. Lives in caller code or `sitevet.toml`. Owned by the user. Not stored by sitevet beyond the run record.
- AuditReport: the result: pages, findings, metrics, scores, site-wide sections, skipped rules, timings, `schema_version`. Returned in memory, written where the user says, stored in history.
- Page result: one audited URL: status, timings, findings, metrics, page score, optional screenshot paths.
- Finding: one rule verdict with evidence.
- Metric: one measurement value or capped table with a status.
- Progress event: transient, never stored.
- Site context: robots.txt, sitemap URLs, link graph. Built per audit, kept inside the report where rules use it.
- History run: one stored audit: ID, URL, start time, duration, options used, sitevet version, overall score, full report JSON. Lives in a local SQLite file. Kept until the user deletes or prunes it.
- Screenshot: PNG files written next to the report when screenshots are on. Owned by the user.
- Fingerprint signatures and the authoritative domain list : bundled read-only data files.

## Integrations

- Target website: outbound HTTP(S) GET and HEAD to the audited origin, plus a check request for each external link. No auth.
- Local Chrome or Chromium: controlled over the DevTools protocol when the browser feature is on. Path auto-detected or set with `chrome_path`.
- axe-core: JavaScript bundled into the crate and injected into rendered pages.
- Google PageSpeed Insights API v5: outbound only when `psi_key` is set. API key auth. Rate-limited by Google, so capped by `psi_max_pages`. Stage 4 must look up the current quota.

## Non-functional requirements

- NFR-01 Performance: a 100-page static audit (no browser) of a typical site finishes in under 60 s on a 16 GB M1 over a 100 Mbit connection, with the site's response time as the main cost. Rule and collector CPU time is under 50 ms per page on average.
- NFR-02 Memory: a 1,000-page static audit stays under 500 MB resident memory.
- NFR-03 Privacy: no telemetry, no update pings, no network requests except to the audited site, the external links it checks, and PSI when a key is set. History stays on the local disk.
- NFR-04 Politeness: at most `concurrency` requests in flight per host (default 5), and an optional per-host delay. The default user agent names sitevet and links to the repo.
- NFR-05 Scale: up to 10,000 pages per audit, HTML pages up to 10 MB each (larger pages are truncated and flagged), report tables capped.
- NFR-06 Platforms: library builds on stable Rust for all three desktop OSes (MSRV set in stage 4). CLI binaries for the five targets in FR-DIST-02.
- NFR-07 Accessibility and locale: HTML report at WCAG 2.2 AA, CLI per FR-CLI-05. Tool text in English. Page analysis in any language.
- NFR-08 Dependency weight: the browser, history and media code all sit behind cargo features that are off by default. The CLI binary without browser support is under 25 MB.
- NFR-09 API stability: the library follows semver from 1.0. The JSON report carries `schema_version`, and any breaking change to it bumps that version.
- NFR-10 Licensing: sitevet is MIT. Every dependency, bundled script (axe-core) and data file has a licence compatible with distributing an MIT binary.
- NFR-11 Test coverage: every rule and collector has at least one passing and one failing fixture test, run offline against local fixtures.

## Out of scope for MVP

- **Login and auth for staging sites** (basic auth, headers, cookies, login forms) : chosen out of v1. Could come back in v1.1 as headers plus basic auth.
- MCP server: v1.1, as a thin wrapper over the stable API.
- **Phase 10** (backlinks, keyword gaps, index coverage, integrity heuristics such as malware patterns and leaked secrets, ad-blocker impact) : needs paid keys or carries false-positive risk. v2.
- Docker image: the static binary covers CI. Later if asked.
- **Translated tool text** (Arabic or other report languages) : v2. Rule IDs let apps translate on their side.
- **GUI, hosted service, dashboard, scheduled runs** : never in this repo. Apps built on the crate do this.
- PDF and CSV export: HTML prints to PDF. JSON converts to CSV. Revisit on request.
- Rendered crawl as the default: stays opt-in (FR-CRAWL-10).

## Appendix A: rule IDs to port (94)

Extracted from the TS package source on 2026-11-10. Where this list and the TS source disagree, the source wins.

- core-seo (11): title-present, title-length, meta-description-present, meta-description-length, single-h1, h1-length, canonical-present, charset-declared, viewport-present, robots-meta-indexable, open-graph-basics
- content (6): heading-hierarchy, word-count, no-empty-headings, keyword-stuffing, text-html-ratio, html-lang
- links (5): broken-internal, broken-external, redirect-chains, weak-anchor-text, external-rel
- images (6): img-alt, img-dimensions, img-modern-format, img-lazy-loading, img-filename-quality, img-empty-src
- structured-data (11): jsonld-present, jsonld-valid, jsonld-type, context-declared, duplicate-jsonld, org-or-website, org-logo, org-sameas, article-required, article-image, breadcrumb-valid
- security (12): https, hsts, csp, x-frame-options, x-content-type-options, referrer-policy, permissions-policy, coop, cookie-secure, mixed-content, server-disclosure, x-powered-by
- crawlability (6): robots-exists, robots-valid, sitemap-present, page-indexable, canonical-valid, x-robots-tag
- url-structure (8): length, depth, hyphens, lowercase, ascii, no-spaces, param-count, readable
- social-media (4): twitter-card, og-image-absolute, og-url-canonical, profile-links
- internationalization (2): hreflang-present, hreflang-valid
- legal (4): privacy-policy, terms, cookie-consent, contact
- eeat (4): author, date, outbound-citations, about-page
- analytics (2): tag-present, consent-mode
- performance (12): render-blocking-scripts, stylesheet-count, inline-script-size, text-compression, cache-control, html-weight, lcp, cls, ttfb, dom-size, page-weight, request-count
- accessibility (1, many findings): axe

## Appendix B: measurement IDs to port

Extracted from the TS package's `src/metrics/*.ts` on 2026-11-10. Some IDs are groups that contain the others. The port follows the TS source where it differs from this list. The TS package is still changing (its phase 2 text metrics are in progress), so the M-milestone that ports collectors re-reads the source.

core-seo: page-basics, title-chars, description-chars, h1-chars, h1-count, open-graph, og-table, og-coverage.
content: page-basics, declared-language, hidden-elements, markup-hygiene, inline-styles, body-style-blocks, html-comments, placeholder-links, headings, heading-count, heading-levels, heading-outline, text-statistics, main-words, unique-words, unique-words-percent, stop-words, stop-words-percent, water-percent, avg-sentence-length, chars-no-spaces, readability, flesch-reading-ease, flesch-kincaid-grade, gunning-fog, keywords, stop-word-table, keyword-relevance, keyword-highlights.
crawlability: response, http-status, response-time, directives, robots-meta, x-robots-tag, canonical-url, canonical-match, robots-txt, sitemap-urls.
links: inventory, total, internal-count, external-count, dofollow-count, nofollow-count, duplicate-count, dofollow-table, nofollow-table, duplicate-table.
images: count, coverage, with-alt, without-alt, http-count.
social-media: twitter, twitter-table, twitter-coverage.
internationalization: hreflang, hreflang-count, hreflang-table.
eeat: dates, published-date, updated-date.
performance: document, html-size, script-count, script-tags.

---

### Open questions

None.

### Assumptions confirmed

1. **robots.txt : RESOLVED.** Obeyed by default, blocked URLs listed in the report, `ignore_robots` overrides.
2. **Staging auth : RESOLVED.** None in v1. Public URLs only.
3. **Run comparison : RESOLVED.** Local SQLite history, auto-saved by the CLI, opt-out with `--no-save`. Library gets it behind a `history` feature, off by default. `history`, `show` and `diff` commands.
4. **MCP server : RESOLVED.** v1.1, not v1.
5. **JS-rendered sites : RESOLVED.** Raw HTML crawl by default. Rendered crawl is opt-in with the browser feature.
6. **Crawl defaults : RESOLVED.** 100 pages, depth 5, concurrency 5 per host, 15 s timeout, cap 10,000.
7. **Exit codes : RESOLVED.** 0 ran and passed, 1 threshold failed, 2 could not run.
8. **Should items : RESOLVED.** FR-CRAWL-10, FR-AEO-07, FR-TRUST-04, FR-MEDIA-02, FR-CLI-07 and FR-DIST-03 are Should. Everything else in phases 2 to 9 is Must.
9. **Screenshots : RESOLVED.** Above-the-fold thumbnails embedded as base64 in the HTML report. Full-page shots stay as linked files.
10. **History location : RESOLVED.** OS data folder, overridable with `SITEVET_HOME`.
11. **Speed target : RESOLVED.** NFR-01 as written: 100 pages static in under 60 s on a 16 GB M1.
