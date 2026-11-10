//! Scoring, ported from the TS package's `src/runner.ts` at commit de27aea.

use std::collections::HashMap;

use crate::report::{Finding, PageResult, Severity, Status};

fn severity_weight(s: Severity) -> f64 {
    match s {
        Severity::Error => 10.0,
        Severity::Warning => 4.0,
        Severity::Info => 1.0,
    }
}

const WARN_CREDIT: f64 = 0.25;

/// Severity-weighted share of passed checks, 0 to 100. Info findings do not count. 100 when
/// nothing was scored.
pub fn score_findings<'a>(findings: impl IntoIterator<Item = &'a Finding>) -> u8 {
    let (mut earned, mut possible) = (0.0, 0.0);
    for f in findings {
        let w = severity_weight(f.severity);
        match f.status {
            Status::Info => continue,
            Status::Pass => earned += w,
            Status::Warn => earned += w * WARN_CREDIT,
            Status::Fail => {}
        }
        possible += w;
    }
    if possible == 0.0 { 100 } else { (earned / possible * 100.0).round() as u8 }
}

fn overall_weight(s: Severity) -> f64 {
    match s {
        Severity::Error => 22.0,
        Severity::Warning => 11.0,
        Severity::Info => 1.0,
    }
}

const WARN_FACTOR: f64 = 0.55;

/// Starts at 100 and deducts for every rule that fails or warns, weighted by severity and scaled
/// by the share of pages it affects.
pub fn compute_overall(pages: &[PageResult]) -> u8 {
    let total = pages.len().max(1) as f64;
    // rule ID -> (severity, pages failing, pages warning)
    let mut by_rule: HashMap<&str, (Severity, u32, u32)> = HashMap::new();
    for p in pages {
        let mut worst: HashMap<&str, (Status, Severity)> = HashMap::new();
        for f in &p.findings {
            if matches!(f.status, Status::Pass | Status::Info) {
                continue;
            }
            let replace =
                worst.get(f.rule_id.as_str()).is_none_or(|(s, _)| *s != Status::Fail && f.status == Status::Fail);
            if replace {
                worst.insert(&f.rule_id, (f.status, f.severity));
            }
        }
        for (rule, (status, severity)) in worst {
            let e = by_rule.entry(rule).or_insert((severity, 0, 0));
            if status == Status::Fail { e.1 += 1 } else { e.2 += 1 }
        }
    }
    let penalty: f64 = by_rule
        .values()
        .map(|(s, fail, warn)| {
            let w = overall_weight(*s);
            w * (f64::from(*fail) / total) + w * WARN_FACTOR * (f64::from(*warn) / total)
        })
        .sum();
    (100.0 - penalty).round().max(0.0) as u8
}
