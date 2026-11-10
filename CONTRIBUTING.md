# Contributing to sitevet

Thanks for helping. This guide covers the one thing most contributions touch: adding or changing a rule.

## Setup

You need Rust 1.88 or newer. Every test runs offline against local fixture servers.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

## How rules are organised

Each category has one file in `crates/sitevet/src/rules/`, such as `core_seo.rs` or `security.rs`. A rule is a unit struct in that file, plus one line in the file's `rules()` list. That's the whole change: one rule, one registry line, one test.

Rule IDs are `category/slug` and match the TypeScript package sitevet was ported from, so `core-seo/viewport-present` means the same check in both.

Rules run in one of three stages:

- `Dom`: during the crawl, with the parsed page. Most rules live here.
- `Enriched`: after the crawl, once the page's links have been checked. Use it when a rule needs link status codes.
- `Rendered`: after the browser or PageSpeed Insights stage, for accessibility and Core Web Vitals.

A rule returns one or more results, each with a status (`pass`, `fail`, `warn` or `info`) and a message. Fail and warn results move the score; info results never do. Numbers that are not verdicts belong in a collector (`crates/sitevet/src/metrics/`), not a rule.

## Worked example: `core-seo/viewport-present`

This is a real rule from `crates/sitevet/src/rules/core_seo.rs`, quoted exactly. A test checks that every code block in this section still matches the source, so the example can't drift.

The rule. `dom_rule!` makes a unit struct that implements `Rule` in the `Dom` stage, with the arguments in this order: struct name, ID, title, severity, fix text, then the body. Inside the body, `dom` is the parsed page and `html` is the raw markup. Helpers such as `attr` live in `rules/kit.rs`:

```rust
dom_rule!(
    ViewportPresent,
    "core-seo/viewport-present",
    "Viewport meta present",
    Warning,
    r#"Add <meta name="viewport" content="width=device-width, initial-scale=1"> for mobile rendering."#,
    |cx, dom, html| {
        match attr(dom, r#"meta[name="viewport"]"#, "content").filter(|v| !v.is_empty()) {
            Some(_) => vec![R::pass("Viewport present")],
            None => vec![R::fail("Missing viewport meta")],
        }
    }
);
```

The registry line, in the same file's `rules()` list:

```rust
        Arc::new(ViewportPresent),
```

The test, in the same file's `tests` module. `page()` runs any rule on an HTML snippet served as `https://example.com/x`. Every rule needs at least one passing case and one failing case:

```rust
    fn viewport() {
        let good = page(r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#);
        assert_eq!(good.status("core-seo/viewport-present"), Pass);
        let bad = page("<p>No viewport here</p>");
        assert_eq!(bad.status("core-seo/viewport-present"), Fail);
        assert_eq!(bad.message("core-seo/viewport-present"), "Missing viewport meta");
    }
```

Run it on its own with `cargo test -p sitevet viewport`.

## Rules in your own app

You don't need to fork sitevet to add a check. Implement the public `Rule` trait and pass your rules in `Options::rules`, alongside or instead of `sitevet::all_rules()`. See `examples/custom-rule.rs`:

```bash
cargo run -p sitevet --example custom-rule -- https://example.com
```

## Writing style

Messages, fix text and docs are plain English: short sentences, no em dashes, straight quotes, sentence case headings. A fix says what to do, for example "Add a lang attribute to <html>".
