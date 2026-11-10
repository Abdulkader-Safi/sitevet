//! `sitevet audit <url>`

use std::io::Write;
use std::process::ExitCode;

use anstream::eprintln;
use anyhow::Context;
use futures::StreamExt;
use sitevet::{AuditEvent, AuditReport, Format, MetricBody, MetricCell, Severity, Status};

use crate::args::{AuditArgs, FailOn, ReportFormat};
use crate::config;
use crate::output::{BOLD, DIM, RED, score_word};

pub fn run(args: AuditArgs) -> anyhow::Result<ExitCode> {
    let (mut options, cli) = config::load(args.config.as_deref())?;
    args.apply(&mut options);
    options.validate()?;

    let out = args.out.clone().or(cli.out);
    let format = args.format.or(cli.format).unwrap_or_else(|| from_extension(out.as_deref()));
    let fail_under = args.fail_under.or(cli.fail_under);
    let fail_on = args.fail_on.or(cli.fail_on);
    let quiet = args.quiet;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(options.runtime_threads.max(1))
        .enable_all()
        .build()
        .context("could not start the runtime")?;
    let report = runtime.block_on(audit(&args.url, options, quiet))?;

    let text = sitevet::render(
        &report,
        match format {
            ReportFormat::Json => Format::Json,
            ReportFormat::Md => Format::Markdown,
            ReportFormat::Html => Format::Html,
        },
    );
    match &out {
        Some(path) => std::fs::write(path, text).with_context(|| format!("could not write {}", path.display()))?,
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(text.as_bytes())?;
            stdout.write_all(b"\n")?;
        }
    }
    if !quiet {
        summary(&report, out.as_deref());
    }
    Ok(thresholds(&report, fail_under, fail_on))
}

fn from_extension(out: Option<&std::path::Path>) -> ReportFormat {
    match out.and_then(|p| p.extension()).and_then(|e| e.to_str()) {
        Some("json") => ReportFormat::Json,
        Some("md" | "markdown") => ReportFormat::Md,
        _ => ReportFormat::Html,
    }
}

async fn audit(url: &str, options: sitevet::Options, quiet: bool) -> anyhow::Result<AuditReport> {
    if !quiet {
        eprintln!("{BOLD}sitevet {}{BOLD:#}  {url}", env!("CARGO_PKG_VERSION"));
        eprintln!("{DIM}up to {} pages, depth {}{DIM:#}", options.max_pages, options.max_depth);
        eprintln!("crawling...");
    }
    let mut stream = sitevet::audit_stream(url, options);
    while let Some(event) = stream.next().await {
        if quiet {
            continue;
        }
        match event {
            AuditEvent::CrawlComplete { pages, checks_total, .. } => {
                let s = if pages == 1 { "" } else { "s" };
                eprintln!("crawled {pages} page{s}, running {checks_total} checks");
            }
            AuditEvent::PageComplete { url, index, pages_total, score, .. } => {
                let (word, style) = score_word(score);
                let width = pages_total.to_string().len();
                eprintln!("{index:>width$}/{pages_total}  {url}  {style}{score}/100  {word}{style:#}");
            }
            _ => {}
        }
    }
    Ok(stream.final_report().await?)
}

fn summary(report: &AuditReport, out: Option<&std::path::Path>) {
    let (word, style) = score_word(report.score);
    eprintln!();
    eprintln!("{BOLD}Score{BOLD:#}  {style}{BOLD}{}/100  {word}{BOLD:#}{style:#}", report.score);
    let name_width = report.categories.iter().map(|c| c.category.len()).max().unwrap_or(0);
    for c in &report.categories {
        let (word, style) = score_word(c.score);
        eprintln!(
            "  {:<name_width$}  {style}{:>3}  {word}{style:#}  {DIM}{} fail, {} warn{DIM:#}",
            c.category, c.score, c.fail, c.warn
        );
    }
    let pages = report.pages.len();
    if pages == 1 && internal_links(report) == Some(0.0) {
        eprintln!("1 page audited, no internal links found");
    } else {
        eprintln!("{pages} {} audited", if pages == 1 { "page" } else { "pages" });
    }
    if !report.blocked.is_empty() {
        eprintln!(
            "{} {} blocked by robots.txt",
            report.blocked.len(),
            if report.blocked.len() == 1 { "URL" } else { "URLs" }
        );
    }
    if let Some(path) = out {
        eprintln!("report written to {}", path.display());
    }
}

/// The start page's internal link count, from its links metric.
fn internal_links(report: &AuditReport) -> Option<f64> {
    report.pages.first()?.metrics.iter().find(|m| m.id == "links/internal-count").and_then(|m| match &m.body {
        MetricBody::Value { value: Some(MetricCell::Number(n)), .. } => Some(*n),
        _ => None,
    })
}

/// 1 when a threshold failed, else 0.
fn thresholds(report: &AuditReport, fail_under: Option<u8>, fail_on: Option<FailOn>) -> ExitCode {
    let mut failed = Vec::new();
    if let Some(min) = fail_under
        && report.score < min
    {
        failed.push(format!("score {} is under --fail-under {min}", report.score));
    }
    if let Some(level) = fail_on {
        let hits = report
            .pages
            .iter()
            .flat_map(|p| &p.findings)
            .filter(|f| match level {
                FailOn::Error => f.status == Status::Fail && f.severity == Severity::Error,
                FailOn::Warning => {
                    matches!(f.status, Status::Fail | Status::Warn)
                        && matches!(f.severity, Severity::Error | Severity::Warning)
                }
            })
            .count();
        if hits > 0 {
            let name = if level == FailOn::Error { "error" } else { "warning" };
            failed.push(format!("{hits} findings at --fail-on {name}"));
        }
    }
    if failed.is_empty() {
        return ExitCode::SUCCESS;
    }
    eprintln!("{RED}{BOLD}threshold failed:{BOLD:#}{RED:#} {}", failed.join(", "));
    ExitCode::from(1)
}
