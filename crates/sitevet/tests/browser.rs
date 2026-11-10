#![cfg(feature = "browser")]
#![allow(clippy::unwrap_used)]
//! The rendered stage in a real Chrome. Tests that launch Chrome run only with
//! SITEVET_TEST_CHROME=1 (Linux CI sets it); the missing-browser test always runs.

mod support;

use std::time::Duration;

use futures::StreamExt;
use sitevet::{AuditEvent, Error, Options, Status, audit, audit_stream};
use support::{Fixture, Route, Site, chain_site};

fn chrome_tests() -> bool {
    std::env::var("SITEVET_TEST_CHROME").as_deref() == Ok("1")
}

fn browser_options() -> Options {
    let mut o = Options::default();
    o.browser = true;
    o
}

/// FR-RENDER-05: a chrome_path that points nowhere fails before the crawl, naming the option
/// and how to fix it.
#[tokio::test]
async fn missing_chrome_fails_before_crawling() {
    let fx = Fixture::start(chain_site(3, Duration::ZERO)).await;
    let mut options = browser_options();
    options.chrome_path = Some("/nowhere/chrome".into());
    let err = audit(&fx.url("/"), options).await.unwrap_err();
    assert!(matches!(err, Error::BrowserNotFound { .. }), "{err}");
    let msg = err.to_string();
    assert!(
        msg.contains("/nowhere/chrome (chrome_path)") && msg.contains("Install Google Chrome or Chromium"),
        "{msg}"
    );
    assert!(fx.hits().is_empty(), "nothing was crawled");
}

/// A form without a label, a large heading for LCP, and a banner that shifts the layout.
const RENDER_PAGE: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Rendered fixture page</title></head>
<body><main>
<h1 style="font-size:64px;height:400px;margin:0">Largest paint candidate</h1>
<div id="slot"></div>
<form><input type="text" id="noLabel"></form>
</main>
<script>addEventListener('load', function () { setTimeout(function () {
  var d = document.createElement('div'); d.style.height = '200px'; d.textContent = 'late banner';
  document.getElementById('slot').appendChild(d);
}, 300); });</script>
</body></html>"#;

/// FR-RENDER-01 and FR-RENDER-02: axe-core reports the missing label, and the Core Web
/// Vitals rules get numbers.
#[tokio::test]
async fn axe_and_web_vitals_fill_in() {
    if !chrome_tests() {
        return;
    }
    let fx = Fixture::start(Site::new().page("/", RENDER_PAGE)).await;
    let mut stream = audit_stream(&fx.url("/"), browser_options());
    let mut render = None;
    while let Some(e) = stream.next().await {
        if let AuditEvent::RenderComplete { provider, rendered, .. } = e {
            render = Some((provider, rendered));
        }
    }
    let report = stream.final_report().await.unwrap();
    assert_eq!(render, Some(("browser".to_string(), 1)));
    assert!(report.skipped_rules.is_empty());
    let page = &report.pages[0];
    let find = |id: &str| page.findings.iter().find(|f| f.rule_id == id).unwrap_or_else(|| panic!("no {id}"));
    assert_eq!(find("accessibility/label").status, Status::Fail);
    assert!(page.findings.iter().filter(|f| f.category == "accessibility").count() > 5, "passes count too");
    let lcp = find("performance/lcp");
    assert!(lcp.message.starts_with("LCP ") && lcp.message.ends_with('s'), "{}", lcp.message);
    assert!(find("performance/cls").message.starts_with("CLS "), "{}", find("performance/cls").message);
    assert!(find("performance/dom-size").message.ends_with("DOM nodes"));
}

/// FR-CRAWL-10: links that exist only after scripts run are found with render_crawl.
#[tokio::test]
async fn render_crawl_finds_script_links() {
    if !chrome_tests() {
        return;
    }
    let spa = r#"<!doctype html><html lang="en"><head><title>SPA</title></head><body><div id="app"></div>
<script>document.getElementById('app').innerHTML = '<a href="/from-script">Loaded by JavaScript</a>';</script></body></html>"#;
    let site = Site::new()
        .page("/", spa)
        .route("/from-script", Route::html("<title>Found</title><p>Only scripts link here.</p>"));
    let fx = Fixture::start(site).await;
    let plain = audit(&fx.url("/"), Options::default()).await.unwrap();
    assert_eq!(plain.pages.len(), 1);
    let mut options = browser_options();
    options.render_crawl = true;
    let rendered = audit(&fx.url("/"), options).await.unwrap();
    assert!(
        rendered.pages.iter().any(|p| p.url.ends_with("/from-script")),
        "{:?}",
        rendered.pages.iter().map(|p| &p.url).collect::<Vec<_>>()
    );
}
