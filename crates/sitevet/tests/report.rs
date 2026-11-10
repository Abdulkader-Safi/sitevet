#![allow(clippy::unwrap_used)]
//! Rendering: the JSON Schema, Markdown and HTML reports, and the one-call helpers.

mod support;

use std::path::Path;

use sitevet::{
    AuditReport, Format, Options, audit, audit_score, audit_to_html, audit_to_markdown, render, report_schema,
};
use support::{Fixture, Route, Site};

fn schema_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema/report.schema.json")
}

/// The committed schema is the generated one. `UPDATE_SCHEMA=1 cargo test` rewrites it.
#[test]
fn committed_schema_is_current() {
    if std::env::var_os("UPDATE_SCHEMA").is_some() {
        std::fs::write(schema_path(), report_schema()).unwrap();
    }
    let committed = std::fs::read_to_string(schema_path()).unwrap().replace("\r\n", "\n");
    assert_eq!(committed, report_schema(), "run UPDATE_SCHEMA=1 cargo test -p sitevet --test report");
}

/// Every TS fixture page as one site, so one audit carries every static rule's findings.
fn ts_fixture_site() -> Site {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages");
    let mut index = String::from("<title>Fixture index</title>");
    let mut site = Site::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        index.push_str(&format!(r#"<a href="/{name}">{name}</a>"#));
        site = site.page(&format!("/{name}"), std::fs::read_to_string(&path).unwrap());
    }
    site.page("/", index)
}

/// FR-REPORT-01 and FR-RULE-02: every fixture report, and so every finding the fixture suite
/// produces, validates against the schema.
#[tokio::test]
async fn every_fixture_report_validates() {
    let schema: serde_json::Value = serde_json::from_str(report_schema()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let sites = [
        Site::from_dir("basic"),
        Site::from_dir("arabic"),
        ts_fixture_site(),
        support::chain_site(4, std::time::Duration::ZERO).route("/p2", Route::html("<h1>down</h1>").status(500)),
    ];
    let mut findings = 0;
    for site in sites {
        let fx = Fixture::start(site).await;
        let report = audit(&fx.url("/"), Options::default()).await.unwrap();
        findings += report.pages.iter().map(|p| p.findings.len()).sum::<usize>();
        let json: serde_json::Value = serde_json::from_str(&render(&report, Format::Json)).unwrap();
        let errors: Vec<String> =
            validator.iter_errors(&json).map(|e| format!("{} at {}", e, e.instance_path())).collect();
        assert!(errors.is_empty(), "{}: {errors:#?}", fx.url("/"));
        assert_eq!(json["schema_version"], 1);
    }
    assert!(findings > 1000, "{findings} findings checked");
}

/// The report with run-specific values (port, time, durations) replaced, for snapshots.
fn stable(text: &str, origin: &str) -> String {
    let time = regex::Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z").unwrap();
    let duration = regex::Regex::new(r"in \d+\.\d s").unwrap();
    let ms = regex::Regex::new(r"\b\d+ ms\b").unwrap();
    let text = text.replace(origin, "http://fixture.test");
    let text = time.replace_all(&text, "2026-01-01T00:00:00Z");
    let text = duration.replace_all(&text, "in 1.0 s");
    ms.replace_all(&text, "1 ms").into_owned()
}

async fn basic_report() -> (AuditReport, String) {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let origin = fx.url("");
    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    (report, origin)
}

/// FR-REPORT-02: the Markdown report of the basic fixture site.
#[tokio::test]
async fn markdown_snapshot() {
    let (report, origin) = basic_report().await;
    insta::assert_snapshot!("basic_markdown", stable(&render(&report, Format::Markdown), &origin));
}

/// The HTML report of the basic fixture site, for review and regression.
#[tokio::test]
async fn html_snapshot_and_offline() {
    let (report, origin) = basic_report().await;
    let html = render(&report, Format::Html);
    // FR-REPORT-03: nothing in the file loads from anywhere.
    for needle in ["src=", "<link", "<script", "url(", "@import", "<iframe", "<img"] {
        assert!(!html.contains(needle), "the report contains {needle}");
    }
    assert!(html.contains("prefers-color-scheme: dark"));
    insta::assert_snapshot!("basic_html", stable(&html, &origin));
}

/// FR-REPORT-04: text quoted from a right-to-left page sits in a `dir="auto"` element.
#[tokio::test]
async fn arabic_title_is_marked_for_its_own_direction() {
    let fx = Fixture::start(Site::from_dir("arabic")).await;
    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    let html = render(&report, Format::Html);
    assert!(html.contains(r#"<pre dir="auto"><code>مرحبا بكم</code></pre>"#), "the short title is quoted as evidence");
    let md = render(&report, Format::Markdown);
    assert!(md.contains("مرحبا بكم"));
}

/// Page text cannot break out of the HTML or the Markdown table.
#[tokio::test]
async fn hostile_page_text_is_escaped() {
    let evil = r#"<title>x</title><h1>a | b <script>alert(1)</script></h1>"#;
    let fx = Fixture::start(Site::new().page("/", evil)).await;
    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    let html = render(&report, Format::Html);
    // The parser takes the <script> apart; its text is quoted, escaped, as evidence.
    assert!(!html.contains("<script>"));
    assert!(html.contains("a | b alert(1)"));
    let md = render(&report, Format::Markdown);
    assert!(!md.contains("a | b"), "pipes in page text are escaped");
}

/// FR-API-05: each helper equals `render` on the report `audit` returns for the same site.
#[tokio::test]
async fn helpers_match_render() {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let origin = fx.url("");
    let url = fx.url("/");
    let report = audit(&url, Options::default()).await.unwrap();
    let html = audit_to_html(&url, Options::default()).await.unwrap();
    let md = audit_to_markdown(&url, Options::default()).await.unwrap();
    assert_eq!(stable(&html, &origin), stable(&render(&report, Format::Html), &origin));
    assert_eq!(stable(&md, &origin), stable(&render(&report, Format::Markdown), &origin));
    assert_eq!(audit_score(&url, Options::default()).await.unwrap(), report.score);
}
