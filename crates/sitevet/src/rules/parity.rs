//! The TS package's own tests (`test/rules.test.ts`, `test/metrics.test.ts` at commit de27aea),
//! ported against the same fixtures.

use std::collections::HashSet;

use super::testkit::{Page, fixture, n, page, row, rows, t, value};
use crate::Status;
use crate::report::{MetricCell, ValueStatus};

fn on(file: &str, url: &str) -> Page {
    page(&fixture(file)).url(url)
}

/// Findings of every static rule (Dom and Enriched) in these categories, all categories when empty.
fn findings(p: &Page, only: &[&str]) -> Vec<crate::Finding> {
    let mut options = crate::Options::default();
    options.only = only.iter().map(|s| s.to_string()).collect();
    super::select_rules(&options)
        .into_iter()
        .filter(|r| r.meta().stage != super::Stage::Rendered)
        .flat_map(|r| {
            let meta = r.meta();
            p.run(r.as_ref())
                .into_iter()
                .map(move |res| crate::engine::pipeline::finding(&meta, res, &p.url))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn registry_has_94_unique_rules() {
    let rules = super::all_rules();
    assert_eq!(rules.len(), 94);
    assert_eq!(rules.iter().map(|r| r.meta().id).collect::<HashSet<_>>().len(), 94);
    // Every rule ID sits under its own category.
    assert!(rules.iter().all(|r| r.meta().id.starts_with(&format!("{}/", r.meta().category))));
}

#[test]
fn static_run_skips_browser_rules() {
    let rendered: Vec<&str> = super::all_rules()
        .iter()
        .filter(|r| r.meta().stage == super::Stage::Rendered)
        .map(|r| r.meta().id)
        .collect::<Vec<_>>()
        .into_iter()
        .collect();
    assert_eq!(
        rendered,
        [
            "accessibility/axe",
            "performance/lcp",
            "performance/cls",
            "performance/ttfb",
            "performance/dom-size",
            "performance/page-weight",
            "performance/request-count"
        ]
    );
    let f = findings(&on("good.html", "https://example.com/good"), &[]);
    assert!(!f.iter().any(|f| f.category == "accessibility" || f.rule_id == "performance/lcp"));
}

#[test]
fn good_fixture_passes_the_core_static_categories() {
    let f = findings(
        &on("good.html", "https://example.com/good"),
        &["core-seo", "content", "images", "structured-data", "url-structure"],
    );
    let score = crate::score_findings(&f);
    assert!(score >= 90, "score was {score}");
    let fails: Vec<&str> = f.iter().filter(|f| f.status == Status::Fail).map(|f| f.rule_id.as_str()).collect();
    assert!(fails.is_empty(), "unexpected fails: {fails:?}");
}

#[test]
fn bad_fixture_trips_known_rules() {
    let mut p = on("bad.html", "http://example.com/bad");
    p.links_ok = false;
    let f = findings(&p, &[]);
    let failed: HashSet<&str> = f.iter().filter(|f| f.status == Status::Fail).map(|f| f.rule_id.as_str()).collect();
    for id in [
        "core-seo/meta-description-present",
        "core-seo/single-h1",
        "core-seo/robots-meta-indexable",
        "images/img-alt",
        "images/img-empty-src",
        "security/https",
        "crawlability/page-indexable",
    ] {
        assert!(failed.contains(id), "{id} did not fail");
    }
    let score = crate::score_findings(&f);
    assert!(score < 60, "score was {score}");
}

#[test]
fn select_rules_filters_by_category() {
    let mut options = crate::Options::default();
    options.only = vec!["security".into()];
    assert!(super::select_rules(&options).iter().all(|r| r.meta().category == "security"));
    assert!(super::select_collectors(&options).iter().all(|c| c.meta().category == "security"));
    options.only = vec![];
    options.skip = vec!["security".into()];
    assert!(super::select_rules(&options).iter().all(|r| r.meta().category != "security"));
}

fn metric(p: &Page, id: &str) -> (Option<MetricCell>, Option<ValueStatus>, Option<String>) {
    value(&p.metric(id))
}

#[test]
fn collector_ids_are_unique_and_metrics_carry_categories() {
    let collectors = super::all_collectors();
    assert_eq!(collectors.iter().map(|c| c.meta().id).collect::<HashSet<_>>().len(), collectors.len());
    let metrics = on("good.html", "https://example.com/good").metrics();
    assert!(!metrics.is_empty());
    assert!(metrics.iter().all(|m| !m.category.is_empty() && m.id.starts_with(&format!("{}/", m.category))));
}

#[test]
fn good_fixture_measures_clean() {
    let p = on("good.html", "https://example.com/good");
    assert_eq!(metric(&p, "core-seo/title-chars").0, n(42.0));
    assert_eq!(metric(&p, "core-seo/title-chars").1, Some(ValueStatus::Ok));
    assert_eq!(metric(&p, "core-seo/h1-count").0, n(1.0));
    assert_eq!(metric(&p, "core-seo/h1-count").1, Some(ValueStatus::Ok));
    assert_eq!(metric(&p, "content/declared-language").0, t("en"));
    assert_eq!(metric(&p, "crawlability/http-status").0, n(200.0));
    assert_eq!(metric(&p, "performance/html-size").1, Some(ValueStatus::Ok));
}

#[test]
fn bad_fixture_reports_missing_values() {
    let p = on("bad.html", "http://example.com/bad");
    assert_eq!(metric(&p, "core-seo/title-chars").1, Some(ValueStatus::Warn), "'Hi' is two characters");
    assert_eq!(metric(&p, "core-seo/description-chars").0, None);
    assert_eq!(metric(&p, "core-seo/description-chars").1, Some(ValueStatus::Missing));
    assert_eq!(metric(&p, "core-seo/h1-count").0, n(2.0));
    assert_eq!(metric(&p, "content/declared-language").1, Some(ValueStatus::Missing));
    assert_eq!(metric(&p, "eeat/published-date").1, Some(ValueStatus::Missing));
}

#[test]
fn dates_and_hidden_elements() {
    let p = on("dated.html", "https://example.com/x");
    assert_eq!(metric(&p, "eeat/published-date").0, t("2026-01-15T09:00:00Z"));
    assert_eq!(metric(&p, "eeat/published-date").1, Some(ValueStatus::Ok));
    assert_eq!(metric(&p, "eeat/updated-date").0, t("2026-03-02"), "read out of JSON-LD");
    assert_eq!(metric(&p, "content/hidden-elements").0, n(2.0));
}

#[test]
fn markup_hygiene() {
    let p = on("directives.html", "https://example.com/directives");
    assert_eq!(metric(&p, "content/inline-styles").0, n(2.0));
    assert_eq!(metric(&p, "content/body-style-blocks").0, n(1.0));
    assert_eq!(metric(&p, "content/html-comments").0, n(2.0), "head and body comments both count");
    assert_eq!(metric(&p, "content/placeholder-links").0, n(2.0));
    assert_eq!(metric(&p, "performance/script-tags").0, n(2.0));
    assert_eq!(metric(&p, "performance/script-tags").2.as_deref(), Some("1 external, 1 inline"));
}

#[test]
fn directives_report_resolved_values() {
    let p = on("directives.html", "https://example.com/directives");
    assert_eq!(metric(&p, "crawlability/robots-meta").0, t("index, follow, max-snippet:-1"));
    assert_eq!(metric(&p, "crawlability/robots-meta").1, Some(ValueStatus::Ok));
    assert_eq!(metric(&p, "crawlability/x-robots-tag").1, Some(ValueStatus::Missing));
    assert_eq!(metric(&p, "crawlability/canonical-url").0, t("https://example.com/canonical-target"));
    assert_eq!(metric(&p, "crawlability/canonical-match").0, t("no"));
    assert_eq!(metric(&p, "crawlability/canonical-match").1, Some(ValueStatus::Warn));
    let slash = on("good.html", "https://example.com/good/");
    assert_eq!(metric(&slash, "crawlability/canonical-match").0, t("yes"));
}

#[test]
fn hreflang_table_only_when_present() {
    let p = on("directives.html", "https://example.com/directives");
    assert_eq!(metric(&p, "internationalization/hreflang-count").0, n(3.0));
    assert_eq!(
        metric(&p, "internationalization/hreflang-count").1,
        Some(ValueStatus::Ok),
        "the en alternate self-references"
    );
    assert_eq!(
        rows(&p.metric("internationalization/hreflang-table"))[1],
        row(&["fr", "https://example.com/fr/directives"])
    );
    let good = on("good.html", "https://example.com/good");
    assert!(!good.metrics().iter().any(|m| m.id == "internationalization/hreflang-table"));
}

#[test]
fn link_counters_and_tables() {
    let p = on("links.html", "https://example.com/links");
    assert_eq!(metric(&p, "links/total").0, n(8.0));
    assert_eq!(metric(&p, "links/internal-count").0, n(5.0));
    assert_eq!(metric(&p, "links/external-count").0, n(3.0));
    assert_eq!(metric(&p, "links/dofollow-count").0, n(6.0));
    assert_eq!(metric(&p, "links/nofollow-count").0, n(2.0));
    assert_eq!(metric(&p, "links/total").2.as_deref(), Some("5 internal, 3 external"));
    assert_eq!(metric(&p, "links/duplicate-count").0, n(3.0));
    let dup = rows(&p.metric("links/duplicate-table"));
    assert_eq!(dup[0], row(&["https://example.com/pricing", "Pricing · pricing page · the plans", "3", "internal"]));
    assert_eq!(dup[1], row(&["https://example.com/about", "About", "2", "internal"]));
    let no = rows(&p.metric("links/nofollow-table"));
    assert_eq!(no.len(), 2);
    let text = |r: &Vec<MetricCell>| format!("{:?}", r[0]);
    assert!(no.iter().all(|r| text(r).contains("partner.example.org")));
    assert!(!rows(&p.metric("links/dofollow-table")).iter().any(|r| text(r).contains("partner.example.org")));
    let good = on("good.html", "https://example.com/good");
    let ids: Vec<String> = good.metrics().into_iter().map(|m| m.id).collect();
    assert!(!ids.iter().any(|i| i == "links/nofollow-table" || i == "links/duplicate-table"));
    assert_eq!(metric(&good, "links/duplicate-count").1, Some(ValueStatus::Ok));
}

#[test]
fn heading_counts_and_outline() {
    let p = on("outline.html", "https://example.com/outline");
    assert_eq!(metric(&p, "content/heading-count").0, n(5.0));
    assert_eq!(metric(&p, "content/heading-count").2.as_deref(), Some("H1 1 · H2 3 · H3 1 · H4 0 · H5 0 · H6 0"));
    let levels = rows(&p.metric("content/heading-levels"));
    assert_eq!(levels, [["H1", "1"], ["H2", "3"], ["H3", "1"], ["H4", "0"], ["H5", "0"], ["H6", "0"]].map(|r| row(&r)));
    let outline = rows(&p.metric("content/heading-outline"));
    assert_eq!(outline.len(), 5);
    assert_eq!(outline[0], row(&["H1", "How the engine works", "-"]));
    assert_eq!(outline[1][1], MetricCell::Text("- Fetching".into()), "an H2 sits one step in");
    assert_eq!(outline[2][1], MetricCell::Text("- - Timeouts".into()), "an H3 sits two steps in");
    assert_eq!(outline[3][1], MetricCell::Text("- Scoring".into()));
    assert_eq!(outline[1][2], MetricCell::Text("IMG: 1, UL: 1".into()));
    assert_eq!(outline[3][2], MetricCell::Text("TABLE: 1".into()));
    assert_eq!(outline[2][2], MetricCell::Text("-".into()));
    assert!(outline.iter().any(|r| format!("{:?}", r[1]).contains("(empty)")));
}

#[test]
fn open_graph_and_twitter_tables() {
    let p = on("social.html", "https://example.com/social");
    assert_eq!(metric(&p, "core-seo/og-coverage").0, t("5 of 7"));
    assert_eq!(metric(&p, "core-seo/og-coverage").1, Some(ValueStatus::Ok));
    let og = rows(&p.metric("core-seo/og-table"));
    assert_eq!(og.len(), 7);
    assert_eq!(og[0], row(&["og:title", "Register of Marks", "present"]));
    assert_eq!(og[6], row(&["og:locale", "-", "not set"]));
    let good = on("good.html", "https://example.com/good");
    let url = rows(&good.metric("core-seo/og-table"))
        .into_iter()
        .find(|r| r[0] == MetricCell::Text("og:url".into()))
        .unwrap();
    assert_eq!(url[2], MetricCell::Text("not set".into()));
    let bare = on("bad.html", "http://example.com/bad");
    let image = rows(&bare.metric("core-seo/og-table"))
        .into_iter()
        .find(|r| r[0] == MetricCell::Text("og:image".into()))
        .unwrap();
    assert_eq!(image[2], MetricCell::Text("missing".into()));
    assert_eq!(metric(&bare, "core-seo/og-coverage").1, Some(ValueStatus::Missing));
    assert_eq!(metric(&p, "social-media/twitter-coverage").0, t("2 of 6"));
    assert_eq!(metric(&p, "social-media/twitter-coverage").1, Some(ValueStatus::Ok));
    assert_eq!(metric(&good, "social-media/twitter-coverage").1, Some(ValueStatus::Missing));
    assert_eq!(metric(&good, "social-media/twitter-coverage").2.as_deref(), Some("Falls back to Open Graph"));
}

#[test]
fn alt_coverage() {
    let p = on("social.html", "https://example.com/social");
    assert_eq!(metric(&p, "images/count").0, n(4.0));
    assert_eq!(metric(&p, "images/with-alt").0, n(2.0));
    assert_eq!(metric(&p, "images/without-alt").0, n(1.0));
    assert_eq!(metric(&p, "images/with-alt").2.as_deref(), Some("1 marked decorative, 1 with no alt attribute"));
    assert_eq!(metric(&p, "images/http-count").0, n(1.0));
    assert_eq!(metric(&p, "images/http-count").1, Some(ValueStatus::Bad));
}

/// FR-MET-02: 300 links give a table of at most the cap and a total of 300.
#[test]
fn link_table_is_capped_with_the_true_total() {
    let html: String = (0..300).map(|i| format!(r#"<a href="/p{i}">Page {i}</a>"#)).collect();
    let m = page(&html).metric("links/dofollow-table");
    let crate::MetricBody::Table { rows, total, .. } = m.body else { panic!("not a table") };
    assert_eq!((rows.len(), total), (crate::ROW_CAP, 300));
}

/// FR-EXT-02: every Rust block in CONTRIBUTING.md's worked example is real source, so the
/// example compiles and its test passes with the rest of the suite.
#[test]
fn contributing_example_matches_the_source() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Windows checkouts may carry CRLF line endings.
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap().replace("\r\n", "\n");
    let (guide, source) = (read("../../CONTRIBUTING.md"), read("src/rules/core_seo.rs"));
    let blocks: Vec<&str> = guide.split("```rust\n").skip(1).map(|b| b.split("```").next().unwrap_or("")).collect();
    assert_eq!(blocks.len(), 3);
    for block in blocks {
        assert!(source.contains(block.trim_end()), "CONTRIBUTING.md block not found in core_seo.rs:\n{block}");
    }
}
