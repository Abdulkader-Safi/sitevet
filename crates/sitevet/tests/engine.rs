#![allow(clippy::unwrap_used)]

mod support;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use futures::StreamExt;
use sitevet::{AuditEvent, AuditReport, Error, Status, audit, audit_stream};
use support::{Fixture, Site, chain_site, counting_proxy, test_options};

/// The report as JSON, without the fields that change from run to run.
fn stable(report: &AuditReport) -> serde_json::Value {
    let mut v = serde_json::to_value(report).unwrap();
    v["generated_at"] = serde_json::Value::Null;
    v["duration_ms"] = serde_json::Value::Null;
    for page in v["pages"].as_array_mut().unwrap() {
        page["response_ms"] = serde_json::Value::Null;
    }
    v
}

/// FR-API-01: 3 linked pages give a report with 3 pages, a 0 to 100 score, findings and metrics.
#[tokio::test]
async fn audit_three_linked_pages() {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let mut options = test_options();
    options.max_depth = 1;
    options.crawl_sitemap = false;
    let report = audit(&fx.url("/"), options).await.unwrap();
    assert_eq!(report.pages.len(), 3);
    assert!(report.score <= 100);
    for page in &report.pages {
        assert!(!page.findings.is_empty() && !page.metrics.is_empty(), "{}", page.url);
    }
    let contact = report.pages.iter().find(|p| p.url.ends_with("/contact.html")).unwrap();
    assert!(contact.findings.iter().any(|f| f.rule_id == "test/title" && f.status == Status::Fail));
    assert!(
        contact
            .findings
            .iter()
            .any(|f| f.rule_id == "test/broken-links" && f.evidence.as_deref().unwrap().ends_with("/missing.html"))
    );
    assert_eq!(report.schema_version, 1);
    assert!(report.generated_at.ends_with('Z'));
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("psi_key"));
}

/// FR-API-02 and FR-EVENT-01: event order and counts, and the streamed report equals `audit`'s.
#[tokio::test]
async fn stream_events_in_order_and_report_matches_audit() {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let mut stream = audit_stream(&fx.url("/"), test_options());
    let mut events = Vec::new();
    while let Some(e) = stream.next().await {
        events.push(e);
    }
    let names: Vec<&str> = events
        .iter()
        .map(|e| match e {
            AuditEvent::AuditStart { .. } => "start",
            AuditEvent::PageFetched { .. } => "fetched",
            AuditEvent::CrawlComplete { .. } => "crawl",
            AuditEvent::CheckComplete { .. } => "check",
            AuditEvent::PageComplete { .. } => "page",
            AuditEvent::AuditComplete { .. } => "complete",
            _ => "other",
        })
        .collect();

    // /, /about.html and /orphan.html (sitemap), /contact.html, then /missing.html (404).
    let pages = 5;
    let rules = 2;
    let crawl_at = names.iter().position(|n| *n == "crawl").unwrap();
    assert_eq!(names[0], "start");
    assert_eq!(names[1..crawl_at], vec!["fetched"; pages]);
    assert_eq!(*names.last().unwrap(), "complete");
    assert_eq!(names.iter().filter(|n| **n == "check").count(), pages * rules);
    assert_eq!(names.iter().filter(|n| **n == "page").count(), pages);
    assert_eq!(names.len(), 1 + pages + 1 + pages * rules + pages + 1);

    let AuditEvent::CrawlComplete { pages: p, checks_total, .. } = &events[crawl_at] else { unreachable!() };
    assert_eq!((*p as usize, *checks_total as usize), (pages, pages * rules));
    let indexes: Vec<u32> = events
        .iter()
        .filter_map(|e| if let AuditEvent::PageFetched { index, .. } = e { Some(*index) } else { None })
        .collect();
    assert_eq!(indexes, [1, 2, 3, 4, 5]);
    let done: Vec<u32> = events
        .iter()
        .filter_map(|e| if let AuditEvent::CheckComplete { checks_done, .. } = e { Some(*checks_done) } else { None })
        .collect();
    assert_eq!(done, (1..=10).collect::<Vec<_>>());
    // Each page's checks come before its PageComplete.
    for (i, e) in events.iter().enumerate() {
        if let AuditEvent::PageComplete { url, findings, .. } = e {
            let checks = events[..i]
                .iter()
                .filter(|c| matches!(c, AuditEvent::CheckComplete { url: u, .. } if u == url))
                .count();
            assert_eq!(checks, rules, "{url}");
            assert!(*findings >= 2);
        }
    }

    let AuditEvent::AuditComplete { report, checks, .. } = events.last().unwrap() else { unreachable!() };
    assert_eq!(*checks, 10);
    let direct = audit(&fx.url("/"), test_options()).await.unwrap();
    assert_eq!(stable(report), stable(&direct));
}

/// FR-EVENT-02: events are Send, cross a std channel and serialise to JSON.
#[test]
fn events_are_send_and_serialise() {
    fn assert_send_clone<T: Send + Clone + 'static>() {}
    assert_send_clone::<AuditEvent>();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        tx.send(AuditEvent::AuditStart { url: "https://example.com/".into(), max_pages: 100 }).unwrap()
    })
    .join()
    .unwrap();
    let json = serde_json::to_string(&rx.recv().unwrap()).unwrap();
    assert_eq!(json, r#"{"type":"audit_start","url":"https://example.com/","max_pages":100}"#);
}

/// FR-API-03: cancelling a 100-page audit mid-crawl closes every connection within 1 s.
#[tokio::test]
async fn cancel_and_drop_close_connections_within_a_second() {
    let fx = Fixture::start(chain_site(100, Duration::from_millis(300))).await;
    let (base, open) = counting_proxy(*fx.server.address()).await;

    // Cancel.
    let mut stream = audit_stream(&format!("{base}/"), test_options());
    while let Some(e) = stream.next().await {
        if matches!(e, AuditEvent::PageFetched { index: 3, .. }) {
            break;
        }
    }
    assert!(open.load(Ordering::SeqCst) > 0, "the crawl should be mid-flight");
    let t = Instant::now();
    stream.cancel();
    assert!(matches!(stream.final_report().await, Err(Error::Cancelled)));
    wait_closed(&open, t).await;

    // Drop.
    let mut stream = audit_stream(&format!("{base}/"), test_options());
    while let Some(e) = stream.next().await {
        if matches!(e, AuditEvent::PageFetched { index: 3, .. }) {
            break;
        }
    }
    let t = Instant::now();
    drop(stream);
    wait_closed(&open, t).await;

    // Dropping the `audit` future.
    let t = tokio::time::timeout(Duration::from_millis(900), audit(&format!("{base}/"), test_options())).await;
    assert!(t.is_err(), "the audit should still be running");
    wait_closed(&open, Instant::now()).await;
}

async fn wait_closed(open: &std::sync::atomic::AtomicUsize, since: Instant) {
    while open.load(Ordering::SeqCst) > 0 {
        assert!(
            since.elapsed() < Duration::from_secs(1),
            "{} connections still open after 1 s",
            open.load(Ordering::SeqCst)
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// A rule that panics becomes an info finding and the audit finishes.
#[tokio::test]
async fn panicking_rule_is_caught() {
    struct Boom;
    impl sitevet::Rule for Boom {
        fn meta(&self) -> sitevet::RuleMeta {
            sitevet::RuleMeta::new("test/boom", "test", "Boom", sitevet::Severity::Error, sitevet::Stage::Dom)
        }
        fn run(&self, _: &sitevet::RuleCx<'_>) -> Vec<sitevet::RuleResult> {
            panic!("kaboom")
        }
    }
    let fx = Fixture::start(chain_site(2, Duration::ZERO)).await;
    let mut options = sitevet::Options::default();
    options.rules = Some(vec![std::sync::Arc::new(Boom)]);
    let report = audit(&fx.url("/"), options).await.unwrap();
    let f = &report.pages[0].findings[0];
    assert_eq!((f.status, f.severity), (Status::Info, sitevet::Severity::Info));
    assert_eq!(f.message, "rule crashed: kaboom");
    assert_eq!(report.score, 100);
}

/// FR-RULE-04: `only = ["security"]` runs only security rules and collectors.
#[tokio::test]
async fn only_filters_rules_and_collectors() {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let mut options = sitevet::Options::default();
    options.only = vec!["security".into()];
    let report = audit(&fx.url("/"), options).await.unwrap();
    assert_eq!(report.categories.iter().map(|c| c.category.as_str()).collect::<Vec<_>>(), ["security"]);
    for page in &report.pages {
        assert!(page.findings.iter().all(|f| f.category == "security"));
        assert!(page.metrics.iter().all(|m| m.category == "security"));
    }
}

/// FR-MET-01 and FR-SCORE-02: the built-in rules run on a real crawl, and the scores are the
/// same with every collector on, with none, and with info-only rules added.
#[tokio::test]
async fn collectors_and_info_findings_never_move_the_score() {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let with = audit(&fx.url("/"), sitevet::Options::default()).await.unwrap();
    let mut options = sitevet::Options::default();
    options.collectors = Some(vec![]);
    let without = audit(&fx.url("/"), options).await.unwrap();
    assert!(with.pages.iter().all(|p| !p.metrics.is_empty()));
    assert!(without.pages.iter().all(|p| p.metrics.is_empty()));
    assert!(with.pages[0].findings.len() > 80, "the built-in rules ran: {}", with.pages[0].findings.len());

    struct InfoOnly;
    impl sitevet::Rule for InfoOnly {
        fn meta(&self) -> sitevet::RuleMeta {
            sitevet::RuleMeta::new("test/note", "core-seo", "A note", sitevet::Severity::Error, sitevet::Stage::Dom)
        }
        fn run(&self, _: &sitevet::RuleCx<'_>) -> Vec<sitevet::RuleResult> {
            vec![sitevet::RuleResult::info("just a note")]
        }
    }
    let mut rules = sitevet::all_rules();
    rules.push(std::sync::Arc::new(InfoOnly));
    let mut options = sitevet::Options::default();
    options.rules = Some(rules);
    let noted = audit(&fx.url("/"), options).await.unwrap();

    let scores = |r: &AuditReport| {
        (
            r.score,
            r.categories.iter().map(|c| (c.category.clone(), c.score)).collect::<Vec<_>>(),
            r.pages.iter().map(|p| p.score).collect::<Vec<_>>(),
        )
    };
    assert_eq!(scores(&with), scores(&without));
    assert_eq!(scores(&with), scores(&noted));
}

/// FR-RENDER-04: without a browser or PSI key the rendered rules are listed as skipped and do
/// not move the score.
#[tokio::test]
async fn rendered_rules_are_skipped_with_a_reason() {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let report = audit(&fx.url("/"), sitevet::Options::default()).await.unwrap();
    let ids: Vec<&str> = report.skipped_rules.iter().map(|s| s.rule_id.as_str()).collect();
    assert_eq!(
        ids,
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
    assert!(report.skipped_rules.iter().all(|s| s.reason == "needs browser or psi_key"));
    assert!(
        !report
            .pages
            .iter()
            .flat_map(|p| &p.findings)
            .any(|f| f.category == "accessibility" || f.rule_id == "performance/lcp")
    );
    assert!(sitevet::render(&report, sitevet::Format::Markdown).contains("| needs browser or psi\\_key |"));
}
