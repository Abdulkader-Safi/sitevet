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
    let format = format_of(args.format.or(cli.format), out.as_deref());
    let fail_under = args.fail_under.or(cli.fail_under);
    let fail_on = args.fail_on.or(cli.fail_on);
    let quiet = args.quiet;
    let save = !args.no_save && cli.save.unwrap_or(true);
    // Loaded before the crawl, so a bad run ID or file fails fast.
    let baseline = match args.baseline.as_deref().or(cli.baseline.as_deref()) {
        Some(spec) => crate::commands::history::load_baseline(spec, &args.url)?,
        None => None,
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(options.runtime_threads.max(1))
        .enable_all()
        .build()
        .context("could not start the runtime")?;
    let mut report = runtime.block_on(audit(&args.url, options, quiet))?;
    if let Some((base, run_id, file)) = &baseline {
        let mut d = sitevet::diff(base, &report);
        d.baseline.run_id = *run_id;
        d.baseline.file = file.clone();
        report.changes = Some(d);
    }

    write_report(&report, format, out.as_deref())?;
    // A failed save warns and leaves the exit code alone.
    let run_id = if save {
        match crate::commands::history::open().and_then(|h| Ok(h.save(&report)?)) {
            Ok(id) => Some(id),
            Err(e) => {
                eprintln!("{RED}warning:{RED:#} the run was not saved to history: {e}");
                None
            }
        }
    } else {
        None
    };
    if !quiet {
        summary(&report, out.as_deref(), run_id);
    }
    Ok(thresholds(&report, fail_under, fail_on))
}

/// The format flag, else the `--out` extension, else HTML.
pub fn format_of(flag: Option<ReportFormat>, out: Option<&std::path::Path>) -> ReportFormat {
    flag.unwrap_or_else(|| match out.and_then(|p| p.extension()).and_then(|e| e.to_str()) {
        Some("json") => ReportFormat::Json,
        Some("md" | "markdown") => ReportFormat::Md,
        _ => ReportFormat::Html,
    })
}

/// Renders to `out`, or stdout.
pub fn write_report(report: &AuditReport, format: ReportFormat, out: Option<&std::path::Path>) -> anyhow::Result<()> {
    let format = match format {
        ReportFormat::Json => Format::Json,
        ReportFormat::Md => Format::Markdown,
        ReportFormat::Html => Format::Html,
    };
    let text = sitevet::render(report, format);
    match out {
        Some(path) => std::fs::write(path, text).with_context(|| format!("could not write {}", path.display()))?,
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(text.as_bytes())?;
            stdout.write_all(b"\n")?;
        }
    }
    Ok(())
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
            AuditEvent::RenderComplete { provider, rendered, pages, .. } => {
                let via = if provider == "psi" { "PageSpeed Insights" } else { "the browser" };
                eprintln!("rendered {rendered} of {pages} pages with {via}");
            }
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

fn summary(report: &AuditReport, out: Option<&std::path::Path>, run_id: Option<i64>) {
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
    if !report.skipped_rules.is_empty() {
        let n = report.skipped_rules.len();
        let reason = &report.skipped_rules[0].reason;
        eprintln!("{n} {} skipped ({reason})", if n == 1 { "rule" } else { "rules" });
    }
    if let Some(d) = &report.changes {
        let base = match (d.baseline.run_id, &d.baseline.file) {
            (Some(id), _) => format!("run {id}"),
            (None, Some(f)) => f.clone(),
            (None, None) => "the baseline".into(),
        };
        let change = if d.score_change > 0 { format!("+{}", d.score_change) } else { d.score_change.to_string() };
        eprintln!("changes since {base}: score {change}, {} fixed, {} new issues", d.fixed.len(), d.new_failures.len());
        if d.not_compared > 0 {
            eprintln!("{} issues not compared: their checks ran in only one of the two runs", d.not_compared);
        }
    }
    if let Some(path) = out {
        eprintln!("report written to {}", path.display());
    }
    if let Some(id) = run_id {
        eprintln!("saved as run {id} (sitevet show {id})");
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
