#![allow(clippy::unwrap_used)]
//! FR-RENDER-03: PageSpeed Insights against a recorded response (the PSI v5 shape, trimmed to
//! the audits sitevet reads). One test in its own binary, because it points the library at the
//! fixture through the SITEVET_PSI_ENDPOINT environment variable.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures::StreamExt;
use sitevet::{AuditEvent, Options, Status, audit_stream};
use support::{Fixture, chain_site};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const RECORDED: &str = include_str!("fixtures/psi/runPagespeed.json");

/// 429 with Retry-After: 1 on the first call, the recorded response after that.
struct RateLimitedOnce(AtomicUsize);

impl Respond for RateLimitedOnce {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            ResponseTemplate::new(429).insert_header("retry-after", "1")
        } else {
            ResponseTemplate::new(200).set_body_raw(RECORDED, "application/json")
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn psi_fills_the_rendered_rules_for_the_first_five_pages() {
    let psi = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/runPagespeed"))
        .and(query_param("key", "test-key"))
        .and(query_param("strategy", "mobile"))
        .respond_with(RateLimitedOnce(AtomicUsize::new(0)))
        .mount(&psi)
        .await;
    // SAFETY: this is the only test in this binary and it runs on a current-thread runtime, so
    // nothing else reads or writes the environment while the variable is set.
    #[allow(unsafe_code)]
    unsafe {
        std::env::set_var("SITEVET_PSI_ENDPOINT", format!("{}/runPagespeed", psi.uri()));
    }

    let site = Fixture::start(chain_site(20, Duration::ZERO)).await;
    let mut options = Options::default();
    options.max_depth = 20;
    options.psi_key = Some("test-key".into());
    let mut stream = audit_stream(&site.url("/"), options);
    let mut render_event = None;
    let mut progress = Vec::new();
    while let Some(e) = stream.next().await {
        match e {
            AuditEvent::PageRendered { index, pages, ok, .. } => {
                assert!(render_event.is_none(), "PageRendered comes before RenderComplete");
                assert!(ok);
                progress.push((index, pages));
            }
            AuditEvent::RenderComplete { provider, rendered, pages, .. } => {
                render_event = Some((provider, rendered, pages))
            }
            _ => {}
        }
    }
    let report = stream.final_report().await.unwrap();

    assert_eq!(render_event, Some(("psi".to_string(), 5, 20)));
    assert_eq!(progress, (1..=5).map(|i| (i, 5)).collect::<Vec<_>>());
    // 5 pages of 20, plus one retry after the 429.
    assert_eq!(psi.received_requests().await.unwrap().len(), 6);
    assert_eq!(report.pages.len(), 20);
    assert!(report.skipped_rules.is_empty(), "the rendered rules ran, on 5 pages");

    let first = &report.pages[0];
    let find = |id: &str| first.findings.iter().find(|f| f.rule_id == id).unwrap_or_else(|| panic!("no {id}"));
    assert_eq!(find("performance/lcp").message, "LCP needs work (3.18s)");
    assert_eq!(find("performance/cls").message, "CLS 0.061");
    assert_eq!(find("performance/ttfb").message, "TTFB 412ms");
    let label = find("accessibility/label");
    assert_eq!(label.status, Status::Fail);
    assert_eq!(find("accessibility/image-alt").status, Status::Pass);
    assert!(!first.findings.iter().any(|f| f.rule_id.contains("video-caption") || f.rule_id.contains("tab-order")));
    // Pages past psi_max_pages get no rendered findings.
    assert!(!report.pages[10].findings.iter().any(|f| f.rule_id == "performance/lcp"));
    // The key never reaches the report.
    assert!(!sitevet::render(&report, sitevet::Format::Json).contains("test-key"));
}
