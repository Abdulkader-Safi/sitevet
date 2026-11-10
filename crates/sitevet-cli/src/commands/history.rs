//! `sitevet history`, `show` and `diff`, and the history helpers `audit` uses.

use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use anstream::eprintln;
use anyhow::{Context, bail};
use sitevet::{AuditReport, History, ReportDiff, RunId};

use crate::args::{DiffArgs, DiffFormat, HistoryAction, HistoryArgs, ShowArgs};
use crate::commands::audit::{format_of, write_report};
use crate::output::{BOLD, DIM, GREEN, RED};

pub fn open() -> anyhow::Result<History> {
    Ok(History::open(None)?)
}

pub fn run(args: &HistoryArgs) -> anyhow::Result<ExitCode> {
    let history = open()?;
    match &args.action {
        None => list(&history, args.url.as_deref(), args.limit),
        Some(HistoryAction::Delete { id, yes }) => {
            let run = history.summary(*id)?;
            if !confirm(&format!("Delete run {id} ({}, {})?", run.start_url, short_date(&run.started_at)), *yes)? {
                eprintln!("nothing deleted");
                return Ok(ExitCode::SUCCESS);
            }
            history.delete(*id)?;
            eprintln!("deleted run {id}");
            Ok(ExitCode::SUCCESS)
        }
        Some(HistoryAction::Prune { keep, yes }) => {
            let total = history.list(None, usize::MAX >> 1)?.len();
            let doomed = total.saturating_sub(*keep);
            if doomed == 0 {
                eprintln!("{total} {} saved, nothing to prune", runs(total));
                return Ok(ExitCode::SUCCESS);
            }
            if !confirm(&format!("Delete {doomed} older {} and keep the newest {keep}?", runs(doomed)), *yes)? {
                eprintln!("nothing deleted");
                return Ok(ExitCode::SUCCESS);
            }
            let removed = history.prune(*keep)?;
            eprintln!("deleted {removed} {}", runs(removed));
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn runs(n: usize) -> &'static str {
    if n == 1 { "run" } else { "runs" }
}

/// `2026-11-10T09:30:00Z` as `2026-11-10 09:30`.
fn short_date(rfc3339: &str) -> String {
    rfc3339.get(..16).map_or_else(|| rfc3339.to_string(), |s| s.replace('T', " "))
}

/// Asks on stderr unless `yes`. Off a terminal it refuses rather than guess.
fn confirm(question: &str, yes: bool) -> anyhow::Result<bool> {
    if yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        bail!("not deleting without confirmation; pass --yes to confirm");
    }
    eprint!("{question} [y/N] ");
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

fn list(history: &History, url: Option<&str>, limit: usize) -> anyhow::Result<ExitCode> {
    let rows = history.list(url, limit)?;
    if rows.is_empty() {
        match url {
            Some(u) => eprintln!("No runs saved for {u} yet. Run sitevet audit {u}."),
            None => eprintln!("No runs saved yet. Run sitevet audit <url>."),
        }
        return Ok(ExitCode::SUCCESS);
    }
    let id_w = rows.iter().map(|r| r.id.to_string().len()).max().unwrap_or(2).max(2);
    let mut out = format!("{:>id_w$}  {:<16}  {:>5}  {:>5}  URL\n", "ID", "DATE", "SCORE", "PAGES");
    for r in &rows {
        out.push_str(&format!(
            "{:>id_w$}  {:<16}  {:>5}  {:>5}  {}\n",
            r.id,
            short_date(&r.started_at),
            r.score,
            r.pages,
            r.start_url
        ));
    }
    print!("{out}");
    Ok(ExitCode::SUCCESS)
}

pub fn show(args: &ShowArgs) -> anyhow::Result<ExitCode> {
    let history = open()?;
    let report = history.load(args.id).map_err(|e| {
        // A run from a newer sitevet: the summary row still reads.
        match history.summary(args.id) {
            Ok(r) if e.to_string().contains("newer") => {
                eprintln!("{DIM}{e}. Summary only:{DIM:#}");
                println!(
                    "run {}  {}  score {}  {} pages  sitevet {}",
                    r.id,
                    short_date(&r.started_at),
                    r.score,
                    r.pages,
                    r.sitevet_version
                );
                anyhow::anyhow!("cannot render run {}", args.id)
            }
            _ => e.into(),
        }
    })?;
    let out = args.out.as_deref();
    write_report(&report, format_of(args.format, out), out)?;
    if let Some(path) = out {
        eprintln!("report written to {}", path.display());
    }
    Ok(ExitCode::SUCCESS)
}

/// Loads a baseline: a run ID, "last" for this site, or a JSON report file.
/// A baseline report, its run ID when it came from history, and its path when it came from a file.
pub type Baseline = (AuditReport, Option<RunId>, Option<String>);

pub fn load_baseline(spec: &str, url: &str) -> anyhow::Result<Option<Baseline>> {
    if spec == "last" {
        let history = open()?;
        return match history.latest_for(url)? {
            Some(id) => Ok(Some((history.load(id)?, Some(id), None))),
            None => {
                eprintln!("{DIM}no earlier run saved for this site, so no changes are shown{DIM:#}");
                Ok(None)
            }
        };
    }
    if let Ok(id) = spec.parse::<RunId>() {
        return Ok(Some((open()?.load(id)?, Some(id), None)));
    }
    let text = std::fs::read_to_string(spec).with_context(|| format!("could not read the baseline {spec}"))?;
    let report: AuditReport =
        serde_json::from_str(&text).with_context(|| format!("{spec} is not a sitevet JSON report"))?;
    Ok(Some((report, None, Some(spec.to_string()))))
}

pub fn diff(args: &DiffArgs) -> anyhow::Result<ExitCode> {
    let history = open()?;
    let (a, b) = (history.load(args.a)?, history.load(args.b)?);
    let mut d = sitevet::diff(&a, &b);
    d.baseline.run_id = Some(args.a);
    match args.format {
        DiffFormat::Json => println!("{}", serde_json::to_string_pretty(&d)?),
        DiffFormat::Md => {
            let mut report = b;
            report.changes = Some(d);
            let md = sitevet::render(&report, sitevet::Format::Markdown);
            // Only the changes section.
            let start = md.find("## Changes since").unwrap_or(0);
            let end = md[start..].find("\n## Category scores").map_or(md.len(), |e| start + e);
            println!("{}", md[start..end].trim_end());
        }
        DiffFormat::Text => print_text(&d, args.a, args.b, a.score, b.score),
    }
    Ok(ExitCode::SUCCESS)
}

fn print_text(d: &ReportDiff, a: RunId, b: RunId, before: u8, after: u8) {
    let signed = |n: i16| if n > 0 { format!("+{n}") } else { n.to_string() };
    let style = if d.score_change >= 0 { GREEN } else { RED };
    anstream::println!("{BOLD}run {a} -> run {b}{BOLD:#}");
    anstream::println!("score {before} -> {after}  {style}{}{style:#}", signed(d.score_change));
    let width = d.categories.iter().map(|c| c.category.len()).max().unwrap_or(0);
    for c in d.categories.iter().filter(|c| c.before != c.after) {
        let s = |v: Option<u8>| v.map_or_else(|| "-".to_string(), |v| v.to_string());
        let change = match (c.before, c.after) {
            (Some(x), Some(y)) => signed(i16::from(y) - i16::from(x)),
            (None, _) => "new".into(),
            (_, None) => "not run".into(),
        };
        anstream::println!("  {:<width$}  {:>3} -> {:>3}  {change}", c.category, s(c.before), s(c.after));
    }
    for (label, list) in [("fixed", &d.fixed), ("new issues", &d.new_failures)] {
        anstream::println!("{label}: {}", list.len());
        for f in list.iter().take(50) {
            anstream::println!("  {}  {}  {DIM}{}{DIM:#}", f.rule_id, f.url, f.message);
        }
        if list.len() > 50 {
            anstream::println!("  {DIM}and {} more (use --format json for all){DIM:#}", list.len() - 50);
        }
    }
    if !d.pages_added.is_empty() || !d.pages_removed.is_empty() {
        anstream::println!("pages: {} added, {} removed", d.pages_added.len(), d.pages_removed.len());
    }
}
