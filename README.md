# sitevet

A website audit engine in Rust. Point it at a URL and it crawls the site, checks every page against 94+ rules for SEO, security, accessibility and performance. You get back a scored report as HTML, Markdown or JSON.

It ships two ways from one codebase:

- A library crate. Add it to your Rust project, call `audit()`, and get a typed report back. Stream progress events into your own UI, add your own rules, render the report yourself.
- A CLI. `sitevet audit https://example.com` for a quick check in the terminal, or with `--fail-under 85` as a CI step that fails when the score drops.

> [!NOTE]
> sitevet is under construction and nothing is published yet. Build progress is in [`PROGRESS.md`](./PROGRESS.md) and the spec in [`docs/spec/`](./docs/spec/).

## What works today

The library's engine core:

- A breadth-first crawl of same-origin pages with page and depth limits, per-host concurrency, an optional per-host delay, manual redirects and size caps.
- robots.txt obeyed by default, with blocked URLs listed in the report. Sitemaps (and sitemap indexes) seed the crawl.
- A shared link checker that requests each URL once per audit.
- Private-network blocking when the start URL is public.
- Three entry points: `audit()`, `audit_stream()` with typed progress events, and `spawn()` for apps that don't run tokio.
- The 94 rules ported from the TypeScript package, with the same IDs, severities and scoring. 87 run today; the accessibility rule and 6 Core Web Vitals rules need the browser stage, which is planned.
- Per-page measurements (title and description lengths, link and heading tables, Open Graph and Twitter tags, directives, dates) that never touch the score.
- Your own rules through the `Rule` trait. See [`CONTRIBUTING.md`](./CONTRIBUTING.md) and `examples/custom-rule.rs`.

Planned: the CLI, HTML and Markdown reports, run history, the browser stage and everything listed below.

## Where it comes from

sitevet is a Rust rebuild of an earlier TypeScript audit package by the same author, which runs 94 rules across 15 categories. That package stays maintained in its own repo. sitevet starts from the same rule catalog and rule IDs, then adds most of that project's roadmap in its first release.

## What v1 checks

The 94 rules from the TypeScript package, covering core SEO, content, links, images, structured data, security headers, crawlability, URL structure, social tags, hreflang, legal pages, analytics, E-E-A-T signals and performance. With a local Chrome install (or a PageSpeed Insights key), it adds a full axe-core accessibility pass and Core Web Vitals.

On top of that, v1 plans to ship:

- Text statistics, readability scores and keyword tables for pages in any language.
- Answer engine checks: short answers near the top, Q&A structure, AI crawler access, `llms.txt`.
- Site-wide findings: duplicate titles and descriptions, near-duplicate pages, orphan pages, click depth, an internal link graph.
- Trust signals and local SEO, such as author bylines, refund pages, and NAP consistency with LocalBusiness schema.
- Tech stack detection for analytics vendors, ad pixels, the CMS and the CDN.
- Deeper performance timings, mobile checks (tap targets, overflow, console errors), screenshots and an opt-in image size audit.

Every page gets two kinds of output. Findings are verdicts (pass, fail, warn) with the offending markup as evidence, and they move the score. Measurements are numbers and tables, like title length or the full heading outline, and they never touch the score.

## Usage

As a library (works today):

```rust
let report = sitevet::audit("https://example.com", sitevet::Options::default()).await?;
println!("{} pages, score {}", report.pages.len(), report.score);
```

From the terminal (planned, not built yet):

```bash
cargo install sitevet-cli
sitevet audit https://example.com --format html --out report.html
```

Settings can live in an optional `sitevet.toml` (`Options::from_toml_str` reads it today). Its keys match the library options one to one, and command-line flags override it.

## License

[MIT](./LICENSE). Copyright [Abdulkader Safi](https://abdulkadersafi.com).
