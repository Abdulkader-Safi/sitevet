# sitevet

Status: signed off 2026-11-10

## Mission

Give Rust developers a website audit engine they can build products on, and give everyone else a single binary that audits a site from the terminal or CI. Safi uses it inside his own projects first, starting with a Rust desktop audit app built on GPUI and gpui-kit (a separate repo), and it is open to community contributors who add rules.

That mission shapes three downstream decisions:

1. Library first. The CLI is a thin wrapper over the public crate API, so anything the CLI can do, a Rust app can do with the same options struct.
2. Rules are data plus a function. Adding one is a single file and a single registry line, so contributors can add rules without learning the engine.
3. Free and open source, no hosted tier. Apps built on sitevet (including Safi's own) are where any business value sits.

sitevet is a separate project from Safi's earlier TypeScript audit package (called "the TS package" in this spec, local path in `CLAUDE.md`). Both stay maintained in their own repos. sitevet reuses its rule catalog, report concepts and roadmap as the feature baseline, and never edits that repo.

## One-liner

sitevet is a Rust crate and CLI that crawls a website, runs 94+ SEO, content, security, accessibility and performance rules plus measurements, and returns a scored report as HTML, Markdown or JSON, for developers who want to embed audits in their own apps or run them in CI.

## The problem

Developers and small agencies audit client sites often: before a launch, after a redesign, monthly for retainer clients. A 20-page audit by hand (view source, Lighthouse per page, a header checker, a schema validator) takes one to two hours. The tools that do it in one pass are either closed desktop apps or single-page checkers.

What exists today:

- **Screaming Frog SEO Spider.** Desktop crawler, closed source. Paid licence is about £199 per user per year with no monthly option. The free version stops at 500 URLs ([Screaming Frog pricing](https://www.screamingfrog.co.uk/seo-spider/pricing/), [licence page](https://screamingfrog.co.uk/seo-spider/licence/)). There is no library you can link into your own app.
- **Sitebulb.** Desktop and cloud, closed. Lite about $18/month, Pro about $42/month plus $11 per extra user, Cloud from $245/month ([Capterra](https://www.capterra.com/p/169089/Sitebulb/pricing/)). Figures vary across aggregators, so verify before quoting to a client.
- **SiteLint.** Commercial audit platform with a Chrome extension and WordPress plugin. The plugin shows scores only, with details on the hosted platform ([WordPress.org](https://wordpress.org/plugins/sitelint/)).
- **Lighthouse / PageSpeed Insights.** Free, very good per page, but one URL at a time, no crawl, no site-wide findings, and PSI is rate-limited.
- **squirrelscan.** CLI auditor that crawls and runs deterministic rules locally, stores audits in a local database, has an MCP server ([docs](https://docs.squirrelscan.com/cli)). Not a Rust library, has accounts and credits for hosted features.
- **crawlie (Spronta).** Rust, MIT, CLI plus MCP server, 46 SEO and GEO checks with a health score ([glama listing](https://glama.ai/mcp/servers/spronta/crawlie)). It is the closest open-source competitor. It ships binaries, not a documented library API, and has no measurements channel.
- **SiteOne Crawler, Scoutly, crawlkit.** Open-source crawlers focused on link health, performance or crawl infrastructure ([SiteOne](https://crawler.siteone.io/introduction/overview/), [crawlkit docs](https://docs.rs/crawlkit-engine)). None covers the full SEO, security, accessibility, E-E-A-T and answer-engine catalog.

The structural flaw they share: the audit engine is locked inside an app (desktop, SaaS or a binary). You cannot `cargo add` it, pass your own rules, stream progress into your own UI and get a typed report back.

The gap: an open-source Rust crate with a large rule catalog, a separate measurements channel, streaming progress events and pluggable rules, plus a CLI that is just that crate on the command line.

## The value

The "after" picture. Safi is building a desktop audit app in Rust with GPUI and gpui-kit. He adds `sitevet = "1"` to `Cargo.toml`, calls `sitevet::audit_stream("https://client.com", opts)` from a background task, and forwards `PageComplete` events to the app window to drive a live progress list. Forty seconds later he has a typed `AuditReport` for 50 pages: overall score, per-category scores, findings with evidence, plus measurement tables (duplicate titles across the site, link graph, readability, detected tech stack). The app shows it in its own GPUI views, and exports the built-in HTML with `render(&report, Format::Html)` when a client wants a file.

When he only needs a quick check, he skips the app and runs `sitevet audit https://client.com` in a terminal. The same afternoon, a GitHub Action runs `sitevet audit https://staging.client.com --fail-under 85` on every deploy and blocks the merge when the score drops.

The key interaction-model difference from crawlie, the closest competitor: sitevet is a library with a CLI on top, not a CLI with nothing underneath. You get the engine, the rule trait and the event stream inside your own program.

Versus crawlie, the user gets:

- A documented, semver-stable Rust API (`audit`, `audit_stream`, `render`, `Rule` and `Collector` traits) : lets developers build apps on it, which is Safi's main use.
- 94 rules at launch on top of the roadmap phases, versus 46 checks : one tool replaces Lighthouse plus a header checker plus a schema validator.
- Measurements separate from findings (numbers and tables that never move the score) : reports can be detailed without diluting what a failure means.
- Local headless Chromium for axe-core accessibility and Core Web Vitals, with PageSpeed Insights as the no-browser fallback : real WCAG and CWV data in the same report.
- Site-wide findings (duplicates, orphans, link graph, click depth) : what agencies expect from a paid crawler.

What it deliberately is not:

- Not a hosted service or dashboard. The JSON report exists so apps (Safi's or anyone's) can build that.
- Not a keyword research, rank tracking or backlink tool. Those need paid third-party data. Roadmap phase 10 (external providers and site integrity heuristics) stays out of v1.
- Not a desktop GUI. Safi's GPUI desktop app is a separate project that consumes the crate.
- Not a replacement for the TypeScript package. Both exist side by side.

## Product category

Technical SEO and website quality auditing, developer tooling segment. It competes with Screaming Frog and Sitebulb on rule coverage, with Lighthouse on per-page quality, and with crawlie and squirrelscan on being a free local CLI. The white space it sits in: embeddable engine. No widely used open-source Rust crate exposes a full audit catalog as a library API.

## Business model and licence

Free and open source. No paid tier, no accounts, no telemetry. Value for Safi comes from the apps and client work built on top of it.

Licence: MIT, matching the TypeScript package. Copyright Abdulkader Safi.

## Platforms

Day one:

- **Library:** any target where `tokio` and `reqwest` build. Tested on macOS (arm64), Linux (x86_64) and Windows (x86_64).
- **CLI:** prebuilt binaries for macOS arm64 and x86_64, Linux x86_64 and arm64 (static musl builds), Windows x86_64. Install via `cargo install sitevet`, GitHub Releases and a shell installer. Homebrew tap is a Should.
- **Rendered checks** (accessibility, Core Web Vitals, mobile extras) need a Chrome or Chromium binary on the machine, or a PageSpeed Insights API key instead.

Later: Docker image for CI users without Rust (not in v1, the static binary covers most CI).

## What success looks like

One year after v1 (around November 2027):

- Safi's GPUI desktop audit app ships on sitevet, and he uses the CLI for quick audits and at least one CI check on a site he maintains.
- Community: 500 GitHub stars, 5,000 crates.io downloads, at least 5 outside contributors who merged a rule.
- Speed: a 50-page static audit finishes in under 30 seconds on an M1 with 16 GB over a normal connection, versus about the same in the TS package for 20 pages.
- Coverage: 94 parity rules plus the roadmap phases 2 to 9 shipped, every rule with a fixture test.

## UI and product references

sitevet has no GUI. Its visible surfaces are the CLI output and the HTML report.

- **TS package HTML report** (`src/report.ts` in the TS package, run `npm run example` there): keep the self-contained single file, overall score badge, per-category sections, colour-coded findings, metric tables with "showing N of M". sitevet gets its own neutral visual identity, with no company branding.
- **TS docs site** (the TS package's rspress docs, `docs/` in that repo): keep the roadmap phase structure and the measurement catalog layout for sitevet's docs.
- **TS streaming example** (`examples/html-report.mjs`): keep the `1/20  https://...  95/100` per-page progress line as the CLI's default progress output.
- **cargo / ripgrep CLI style:** coloured status on stderr, report on stdout or `--out`, exit codes that CI can read.

---

### Open questions

None.

### Assumptions confirmed

1. **Packaging : RESOLVED.** One library crate plus a CLI binary built on it. "Container" meant crate. No Docker in v1.
2. **Name : RESOLVED.** `sitevet`. `sitelint` was rejected because SiteLint is an existing commercial audit product. `sitevet` is free on crates.io (checked 2026-11-10) and no product clash was found.
3. **TS package : RESOLVED.** Separate repo, stays active, Safi keeps working on it. sitevet never touches it.
4. **v1 scope : RESOLVED.** Full parity with TS 1.0.2 (phases 0 and 1, including local Chromium and PSI) plus roadmap phases 2, 3, 4, 5, 6, 7, 8 and 9. Phase 10 is out of v1.
5. **Config file : RESOLVED.** CLI reads an optional `sitevet.toml` that maps one-to-one to the library options struct. CLI flags override it.
6. **Audience : RESOLVED.** Safi's own projects first, open to community contributors.
7. **Licence : RESOLVED.** MIT only, matching the TS package.
8. **Report schema : RESOLVED.** sitevet has its own versioned JSON schema (`schema_version` field) with the same concepts as the TS report (findings, metrics, scores) and the same rule IDs (`core-seo/title-present`). No byte-for-byte compatibility promise.
9. **Success targets : RESOLVED.** 500 GitHub stars, 5,000 crates.io downloads, 5 outside rule contributors, used in Safi's own products, all one year after v1.
10. **First consumer : RESOLVED.** A Rust desktop audit app built with GPUI and gpui-kit, in a separate repo. The CLI covers quick audits without the app. Stage 4 must check how the async engine runs inside a GPUI app (GPUI has its own executor) so the library API fits it.
