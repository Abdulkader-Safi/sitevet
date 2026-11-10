# Users and usage

Status: signed off 2026-11-10

Two primary users with equal weight: the developer who embeds the crate, and the person who runs the CLI. When a choice helps one and hurts the other, the tie-breaker is the rule from stage 1: the CLI is a thin wrapper over the library, so the library API gets designed first and the CLI exposes it without adding behaviour of its own.

## Personas

### Primary: "Omar," the app builder

- **Background:** Rust developer building a product on top of an audit engine, for example a desktop audit app or a reporting service. Writes async Rust daily and knows `tokio`, `serde` and cargo features.
- **What drives them:** wants a typed report he can render in his own UI, progress events he can forward to a window, and the option to add his own rules without forking. Cares about compile time and dependency weight, because the crate ends up inside his binary.
- **Current workaround:** shells out to a Node package or a CLI and parses its JSON, or hand-rolls 20 checks with `reqwest` and `scraper`. Either way he loses typed results and cancellation, plus any streaming progress.
- **Tech comfort:** high. Reads docs.rs, sets feature flags, will read the source when docs are thin.
- **Relationship to dependencies:** "only what I turn on". Chromium and heavy features must sit behind cargo features he opts into.
- **Devices and environment:** macOS (arm64) for development, ships desktop builds for every major OS. Normal broadband.

### Primary: "Lina," the quick auditor

- **Background:** freelance web developer or small-agency lead in the GCC who builds and maintains client sites (WordPress, Next.js, Laravel). Audits a site before launch, after a redesign, and monthly for retainer clients.
- **What drives them:** a scored, client-ready report in one command, so a 20-page audit takes minutes instead of the one to two hours of Lighthouse plus header checkers plus a schema validator.
- **Current workaround:** Screaming Frog free tier (500 URL cap), Lighthouse one page at a time, browser extensions, then pasting results into a doc by hand.
- **Tech comfort:** comfortable in a terminal and can install a binary. Will not compile Rust or edit code. Will copy a `sitevet.toml` from the docs and tweak it.
- **Relationship to cost and privacy:** wants free and local. Client sites and staging URLs should not go to a third-party service unless she turns on PageSpeed Insights herself.
- **Devices and environment:** macOS or Windows laptop, sometimes slow hotel or office Wi-Fi. Client sites often in Arabic, though she reads the report in English.

### Secondary: "Dev," the CI maintainer

- **Background:** sets up the pipeline for a team's marketing site or docs site. Adds `sitevet audit $STAGING_URL --fail-under 85` as a GitHub Actions step.
- **What drives them:** a non-zero exit code when the score drops, readable output in the CI log, and a JSON artifact to diff between runs.
- **Current workaround:** Lighthouse CI on a few URLs, which misses site-wide problems like duplicate titles and broken internal links.
- **Tech comfort:** high on CI, low interest in SEO details.
- **Why secondary:** served almost for free by the CLI if exit codes and output modes are right. Not the beachhead because CI users rarely pick the tool; Lina or Omar picks it and CI follows.
- **Devices and environment:** Linux runners (x86_64 and arm64), no Chrome unless installed in the job.

### Secondary: "Sam," the rule contributor

- **Background:** a developer from the community who wants a check that sitevet lacks, for example a new schema type or a local regulation page.
- **What drives them:** adding one rule in one file, with a fixture test, and getting it merged within a week.
- **Why secondary:** matters for the 5-contributor target in stage 1, but only shows up once Omar and Lina use the tool.

### Not the target

- Non-technical site owners who will not open a terminal. They are served by GUI apps built on the crate. sitevet will not add wizards, interactive prompts or GUI-like features for them.
- SEO teams who want rank tracking, keyword volumes or backlink data. Those need paid third-party data (roadmap phase 10) and stay out of v1.

## Roles and permissions

Single-user local tool. No accounts, no roles, no server. The library runs with the permissions of the app that embeds it, and the CLI with those of the person running it.

## Jobs to be done

- When **Omar builds an audit feature into his app**, he wants to **call one async function with an options struct and receive typed events and a typed report**, so he can **show live progress and render results in his own UI**.
- When **Omar needs a check sitevet lacks**, he wants to **implement a `Rule` or `Collector` trait in his own crate and pass it in**, so he can **extend the audit without forking**.
- When **Omar's user cancels a long crawl**, he wants to **drop or cancel the audit and have it stop within a second**, so the **app stays responsive**.
- When **Lina gets a new client site**, she wants to **run one command and get a self-contained HTML report**, so she can **send it to the client the same day**.
- When **Lina only cares about one area** (security headers before a launch), she wants to **filter by category**, so the **report stays short**.
- When **Lina re-audits after fixes**, she wants to **see the score go up for the categories she worked on**, so she can **show the client progress**.
- When **Dev adds a CI step**, he wants **a stable exit code, plain log output and a JSON file**, so the **build fails on a regression and the artifact explains why**.
- When **Sam adds a rule**, he wants **one file, one registry line and a fixture test pattern to copy**, so the **PR is small and easy to review**.

## Primary user journey

Two primary users, so two short journeys.

### Omar (library)

1. **Discovery:** crates.io search for "seo audit" or "website audit", or the docs.rs page linked from the README.
2. **Setup:** `cargo add sitevet`, optionally `features = ["browser"]`. Under five minutes, including reading the quick start.
3. **First use:** copies the docs example, calls `sitevet::audit(url, Options::default())`, prints `report.score`.
4. **Aha moment:** switches to `audit_stream`, sees `PageComplete` events arrive one by one with per-page scores, and wires them into his UI in about 20 lines.
5. **Optional power-up:** adds his own rule through the `Rule` trait, or turns on the browser feature for axe-core and Core Web Vitals.
6. **Finish:** renders the typed report in his own UI, or calls `render(&report, Format::Html)` for the built-in file.
7. **Ongoing use:** every audit his app runs goes through sitevet. He upgrades when new rules ship.

### Lina (CLI)

1. **Discovery:** a GitHub link, or a post comparing free alternatives to Screaming Frog.
2. **Setup:** one-line shell installer or a binary from GitHub Releases. No Rust toolchain. Under two minutes.
3. **First use:** `sitevet audit https://client.com`. Sees a per-page progress line (`3/20  https://client.com/about  92/100`) and a summary.
4. **Aha moment:** opens `report.html` and finds duplicate titles across 6 pages and a missing HSTS header she never would have checked by hand.
5. **Optional power-up:** `--browser` with Chrome installed, or a PageSpeed key, for accessibility and Core Web Vitals. A `sitevet.toml` per client with `max_pages` and skipped categories.
6. **Finish:** emails the HTML report or pastes the Markdown into a ticket.
7. **Ongoing use:** monthly re-audit for retainer clients, and a quick check before every launch.

## Core loop

**The repeated action:** run an audit -> read the score and the failing findings -> fix the site -> re-run and compare.

**What pulls users back:**

- The score moves after a fix, which makes progress visible to clients.
- Each release adds rules, so a re-run finds things the last run could not.
- For Omar, every audit his own app runs is a sitevet run.

## Language and accessibility

- **Report language:** English only in v1, for the HTML report, the Markdown report, rule messages, fix text and CLI output. Each rule keeps its message and fix text in one place so translation can come later, and consumers can translate on their side using stable rule IDs.
- **Page content analysis:** sitevet audits a website in any language. Every rule and the text engine work on any page, whatever its language. The page language comes from `<html lang>`, falling back to detection from the text. Tokenizing is Unicode-aware, so Arabic, Cyrillic, Greek and other non-Latin scripts count words correctly. Stop words and word grouping (stemming) cover as many languages as the chosen libraries support, with English and Arabic as the tested minimum. Readability uses a published formula for the page language where one exists. For a language without one, the report shows language-neutral numbers (average sentence and word length) and labels the formula as "not available for this language" instead of returning a wrong score.
- **RTL content in reports:** the HTML report itself is English and left-to-right, but any text quoted from a right-to-left page (Arabic, Hebrew, Persian, Urdu) (evidence, titles, headings, link anchors) must display in the right direction (`dir="auto"` on quoted content) and never break the layout.
- **Accessibility of the HTML report:** WCAG 2.2 AA. Status is never shown by colour alone (text label next to every colour), headings in order, readable at 200% zoom.
- **CLI accessibility:** respects `NO_COLOR`, uses status words (PASS, FAIL, WARN) as well as colour, and has a plain non-animated output mode for screen readers and CI logs.
- **Formats:** dates in ISO 8601 (`2026-11-10`), durations in ms or s, sizes in KB with one decimal.

---

### Open questions

None.

### Assumptions confirmed

1. **Primary users : RESOLVED.** Library developer and CLI user are equal primaries. Ties go to library-first design, with the CLI as a thin wrapper.
2. **Report language : RESOLVED.** English only in v1. Messages kept per rule so translation can come later.
3. **Content language : RESOLVED.** The tool's own text is English, but it audits websites in any language. Text analysis adapts to the page language, and only readability falls back to neutral numbers where no formula exists for that language.
4. **Not the target : RESOLVED.** Non-technical site owners (served by GUI apps built on the crate), and SEO teams wanting rank or backlink data.
5. **Accounts : RESOLVED.** None. Single-user local tool.
6. **Install path : RESOLVED.** Shell installer and GitHub Releases binaries are Must. Homebrew tap is Should.
7. **Cancellation : RESOLVED.** Dropping or cancelling an audit stops all network and browser work in under one second.
