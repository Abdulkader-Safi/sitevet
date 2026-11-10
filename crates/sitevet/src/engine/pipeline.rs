//! One audit from start URL to report: crawl, DOM stage, enriched stage, score.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use futures::StreamExt;
use scraper::Html;
use tokio_util::sync::CancellationToken;
use url::Url;

use super::{AuditEvent, Events};
use crate::crawl::{self, Crawl, Fetcher, LinkChecker, RobotsGuard};
use crate::report::{
    AuditReport, CategoryScore, Finding, Metric, MetricBody, PageResult, SkippedRule, Status, ValueStatus,
};
use crate::rules::{self, Collector, Rule, RuleCx, RuleMeta, RuleResult, Stage};
use crate::{Error, LinkStatusMap, Options, PageData, Result, extract, render_stage, score, time};

const EVIDENCE_CAP: usize = 2_000;

/// Runs until done or until `cancel` fires. Dropping the future stops everything too.
pub(super) async fn run(
    url: String,
    options: Options,
    events: Events,
    cancel: CancellationToken,
) -> Result<AuditReport> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(Error::Cancelled),
        r = audit(url, options, &events) => r,
    }
}

/// A rule's results on one page.
struct Outcome {
    meta: RuleMeta,
    results: Vec<RuleResult>,
}

async fn audit(url: String, options: Options, events: &Events) -> Result<AuditReport> {
    let started = Instant::now();
    options.validate()?;
    let invalid = |reason: String| Error::InvalidUrl { url: url.clone(), reason };
    let start = Url::parse(&url).map_err(|e| invalid(e.to_string()))?;
    if !matches!(start.scheme(), "http" | "https") {
        return Err(invalid("only http and https URLs can be audited".into()));
    }
    let start = crawl::normalize(&start);

    let allow_private = match options.allow_private_network {
        Some(allow) => allow,
        None => crawl::host_is_private(&start).await,
    };
    let fetcher = Fetcher::new(
        options.user_agent(),
        start.origin(),
        options.concurrency as usize,
        Duration::from_millis(options.delay_ms),
        Duration::from_millis(options.timeout_ms),
        !allow_private,
    )
    .map_err(|e| Error::StartPageUnreachable { url: url.clone(), source: e.into() })?;
    let fetcher = Arc::new(fetcher);

    // A missing or broken Chrome fails the audit here, before anything is crawled.
    let mut provider = render_stage::prepare(&options, !allow_private).await?;

    events.send(AuditEvent::AuditStart { url: url.clone(), max_pages: options.max_pages }).await;

    let site = Arc::new(crawl::load_site(&fetcher, &start).await);
    let ua = options.user_agent();
    let agent = ua.split(['/', ' ']).next().unwrap_or("sitevet");
    let robots = Arc::new(RobotsGuard::new(&site.robots, agent, !options.ignore_robots));
    let links = LinkChecker::new(fetcher.clone(), robots.clone(), start.origin());

    let (dom_rules, enriched_rules, rendered_rules) = split(rules::select_rules(&options), |r| r.meta().stage);
    let (dom_collectors, enriched_collectors, rendered_collectors) =
        split(rules::select_collectors(&options), |c| c.meta().stage);
    let render_crawl = options.render_crawl;
    let (dom_rules, dom_collectors) = (Arc::new(dom_rules), Arc::new(dom_collectors));

    let crawl_started = Instant::now();
    let origin = start.origin();
    let crawl = Crawl {
        fetcher: &fetcher,
        robots: &robots,
        links: &links,
        start: &start,
        site: &site,
        max_pages: options.max_pages as usize,
        max_depth: options.max_depth,
        concurrency: options.concurrency as usize,
        crawl_sitemap: options.crawl_sitemap,
    };
    let crawled = crawl
        .run(events, |page, body| {
            let (rules, collectors, site, origin) =
                (dom_rules.clone(), dom_collectors.clone(), site.clone(), origin.clone());
            let provider = provider.as_ref();
            async move {
                // render_crawl: read links and content from the DOM after scripts ran.
                let body = match provider {
                    Some(p) if render_crawl && page.error.is_none() => {
                        p.rendered_html(&page.final_url).await.map_or(body, String::into_bytes)
                    }
                    _ => body,
                };
                let fallback = page.clone();
                // The DOM is not Send: it is built, read and dropped inside this one blocking call.
                let job = tokio::task::spawn_blocking(move || {
                    let mut page = page;
                    let html = String::from_utf8_lossy(&body);
                    let dom = Html::parse_document(&html);
                    extract::extract(&dom, &mut page, &origin);
                    let no_links = LinkStatusMap::new();
                    let cx = RuleCx {
                        page: &page,
                        dom: Some(&dom),
                        html: Some(&html),
                        rendered: None,
                        links: &no_links,
                        site: &site,
                    };
                    let out = (run_rules(&rules, &cx), run_collectors(&collectors, &cx));
                    (page, out)
                });
                job.await.unwrap_or_else(|_| {
                    let mut page = fallback;
                    page.error = Some("page extraction crashed".into());
                    (page, (Vec::new(), Vec::new()))
                })
            }
        })
        .await?;

    let crawl_ms = crawl_started.elapsed().as_millis() as u64;
    let pages_total = crawled.pages.len() as u32;

    // The rendered stage, after the crawl, as in the TS package: RenderComplete comes before
    // CrawlComplete, whose check total depends on which pages rendered.
    let mut rendered = std::collections::HashMap::new();
    let mut skipped = Vec::new();
    let skip_all = |reason: &str| -> Vec<SkippedRule> {
        rendered_rules
            .iter()
            .map(|r| SkippedRule { rule_id: r.meta().id.to_string(), reason: reason.to_string() })
            .collect()
    };
    match provider.take() {
        Some(mut p) => {
            let render_started = Instant::now();
            let urls: Vec<String> = crawled.pages.iter().map(|(page, _)| page.url.clone()).collect();
            let out = p.render(&urls).await;
            let name = p.name();
            p.close().await;
            events
                .send(AuditEvent::RenderComplete {
                    provider: name.to_string(),
                    rendered: out.pages.len() as u32,
                    pages: pages_total,
                    ms: render_started.elapsed().as_millis() as u64,
                })
                .await;
            if let Some(reason) = out.failed {
                skipped = skip_all(&format!("the rendered stage stopped: {reason}"));
            }
            rendered = out.pages;
        }
        None => skipped = skip_all(render_stage::NEEDS_RENDERING),
    }

    let static_checks = (dom_rules.len() + enriched_rules.len()) as u32;
    let checks_total = pages_total * static_checks + rendered.len() as u32 * rendered_rules.len() as u32;
    events.send(AuditEvent::CrawlComplete { pages: pages_total, checks_total, ms: crawl_ms }).await;

    let need_links = !enriched_rules.is_empty() || !enriched_collectors.is_empty();
    let (checks_done, pages_done) = (AtomicU32::new(0), AtomicU32::new(0));
    let pages: Vec<PageResult> = futures::stream::iter(crawled.pages)
        .map(|(page, (mut outcomes, mut metrics))| {
            let (enriched_rules, enriched_collectors, site, links) =
                (&enriched_rules, &enriched_collectors, &site, &links);
            let (rendered_rules, rendered_collectors) = (&rendered_rules, &rendered_collectors);
            let data = rendered.get(&page.url);
            let (checks_done, pages_done) = (&checks_done, &pages_done);
            async move {
                let page_started = Instant::now();
                let link_map = if need_links {
                    links.check_all(page.links.iter().filter_map(|l| l.url.as_deref())).await
                } else {
                    LinkStatusMap::new()
                };
                {
                    let cx = RuleCx { page: &page, dom: None, html: None, rendered: None, links: &link_map, site };
                    outcomes.extend(run_rules(enriched_rules, &cx));
                    metrics.extend(run_collectors(enriched_collectors, &cx));
                }
                if let Some(data) = data {
                    let cx =
                        RuleCx { page: &page, dom: None, html: None, rendered: Some(data), links: &link_map, site };
                    outcomes.extend(run_rules(rendered_rules, &cx));
                    metrics.extend(run_collectors(rendered_collectors, &cx));
                }
                let mut findings = Vec::new();
                for o in outcomes {
                    let done = checks_done.fetch_add(1, Ordering::Relaxed) + 1;
                    let status = worst(&o.results);
                    events
                        .send(AuditEvent::CheckComplete {
                            url: page.url.clone(),
                            rule_id: o.meta.id.to_string(),
                            status,
                            checks_done: done,
                            checks_total,
                        })
                        .await;
                    findings.extend(o.results.into_iter().map(|r| finding(&o.meta, r, &page.url)));
                }
                let result = page_result(page, findings, metrics);
                events
                    .send(AuditEvent::PageComplete {
                        url: result.url.clone(),
                        index: pages_done.fetch_add(1, Ordering::Relaxed) + 1,
                        pages_total,
                        score: result.score,
                        findings: result.findings.len() as u32,
                        metrics: result.metrics.len() as u32,
                        ms: page_started.elapsed().as_millis() as u64,
                    })
                    .await;
                result
            }
        })
        .buffered(options.concurrency as usize)
        .collect()
        .await;

    let report = report(url, start, &options, started, pages, crawled.blocked, skipped);
    events
        .send(AuditEvent::AuditComplete {
            score: report.score,
            pages: report.pages.len() as u32,
            checks: checks_done.load(Ordering::Relaxed),
            ms: started.elapsed().as_millis() as u64,
            report: Box::new(report.clone()),
        })
        .await;
    Ok(report)
}

/// Splits by stage into (Dom, Enriched, Rendered).
fn split<T>(items: Vec<T>, stage: impl Fn(&T) -> Stage) -> (Vec<T>, Vec<T>, Vec<T>) {
    let (mut dom, mut enriched, mut rendered) = (Vec::new(), Vec::new(), Vec::new());
    for item in items {
        match stage(&item) {
            Stage::Dom => dom.push(item),
            Stage::Enriched => enriched.push(item),
            Stage::Rendered => rendered.push(item),
        }
    }
    (dom, enriched, rendered)
}

/// Runs each rule under `catch_unwind`. A panic becomes one info result.
fn run_rules(rules: &[Arc<dyn Rule>], cx: &RuleCx<'_>) -> Vec<Outcome> {
    rules
        .iter()
        .map(|rule| {
            let meta = rule.meta();
            let results = catch_unwind(AssertUnwindSafe(|| rule.run(cx))).unwrap_or_else(|panic| {
                let mut r = RuleResult::new(Status::Info, format!("rule crashed: {}", panic_message(&panic)));
                r.severity = Some(crate::Severity::Info);
                vec![r]
            });
            Outcome { meta, results }
        })
        .collect()
}

/// Runs each collector under `catch_unwind`. A panic becomes a missing value with a note.
fn run_collectors(collectors: &[Arc<dyn Collector>], cx: &RuleCx<'_>) -> Vec<Metric> {
    let mut out = Vec::new();
    for c in collectors {
        let meta = c.meta();
        match catch_unwind(AssertUnwindSafe(|| c.collect(cx))) {
            Ok(inputs) => out.extend(inputs.into_iter().map(|m| Metric {
                id: m.id,
                category: meta.category.to_string(),
                label: m.label,
                status: m.status,
                note: m.note,
                body: m.body,
            })),
            Err(panic) => out.push(Metric {
                id: meta.id.to_string(),
                category: meta.category.to_string(),
                label: meta.id.to_string(),
                status: Some(ValueStatus::Missing),
                note: Some(format!("Collector errored: {}", panic_message(&panic))),
                body: MetricBody::Value { value: None, unit: None },
            }),
        }
    }
    out
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

/// fail before warn before info before pass. No results counts as pass.
fn worst(results: &[RuleResult]) -> Status {
    let rank = |s: Status| match s {
        Status::Fail => 0,
        Status::Warn => 1,
        Status::Info => 2,
        _ => 3,
    };
    results.iter().map(|r| r.status).min_by_key(|s| rank(*s)).filter(|s| rank(*s) < 3).unwrap_or(Status::Pass)
}

pub(crate) fn finding(meta: &RuleMeta, r: RuleResult, page_url: &str) -> Finding {
    let rule_id = r.rule_id.unwrap_or_else(|| meta.id.to_string());
    let evidence =
        r.evidence.map(|e| if e.chars().count() > EVIDENCE_CAP { e.chars().take(EVIDENCE_CAP).collect() } else { e });
    let key = finding_key(&rule_id, page_url, evidence.as_deref());
    Finding {
        category: meta.category.to_string(),
        title: r.title.unwrap_or_else(|| meta.title.to_string()),
        status: r.status,
        severity: r.severity.unwrap_or(meta.severity),
        message: r.message,
        details: r.details,
        fix: r.fix.or_else(|| meta.fix.map(str::to_string)),
        evidence,
        key,
        rule_id,
    }
}

/// FNV-1a, 64 bit: stable across Rust versions, unlike the std hasher.
fn finding_key(rule_id: &str, page_url: &str, evidence: Option<&str>) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in [rule_id, "\n", page_url, "\n", evidence.unwrap_or("")] {
        for b in part.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

fn page_result(page: PageData, findings: Vec<Finding>, metrics: Vec<Metric>) -> PageResult {
    PageResult {
        score: score::score_findings(&findings),
        url: page.url,
        final_url: page.final_url,
        status: page.status,
        error: page.error,
        depth: page.depth,
        redirect_chain: page.redirect_chain,
        response_ms: page.response_ms,
        html_bytes: page.html_bytes,
        truncated: page.truncated,
        findings,
        metrics,
    }
}

fn report(
    url: String,
    start: Url,
    options: &Options,
    started: Instant,
    pages: Vec<PageResult>,
    blocked: Vec<crate::BlockedUrl>,
    skipped_rules: Vec<SkippedRule>,
) -> AuditReport {
    let mut by_category: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
    for f in pages.iter().flat_map(|p| &p.findings) {
        by_category.entry(&f.category).or_default().push(f);
    }
    let count = |fs: &[&Finding], s: Status| fs.iter().filter(|f| f.status == s).count() as u32;
    let categories = by_category
        .iter()
        .map(|(category, fs)| CategoryScore {
            category: category.to_string(),
            score: score::score_findings(fs.iter().copied()),
            pass: count(fs, Status::Pass),
            fail: count(fs, Status::Fail),
            warn: count(fs, Status::Warn),
        })
        .collect();
    let mut summary = options.clone();
    summary.psi_key = None;
    summary.rules = None;
    summary.collectors = None;
    AuditReport {
        schema_version: 1,
        sitevet_version: env!("CARGO_PKG_VERSION").to_string(),
        final_url: pages.first().map_or_else(|| start.to_string(), |p| p.final_url.clone()),
        start_url: url,
        generated_at: time::now_rfc3339(),
        duration_ms: started.elapsed().as_millis() as u64,
        options: summary,
        score: score::compute_overall(&pages),
        categories,
        pages,
        blocked,
        skipped_rules,
        changes: None,
    }
}
