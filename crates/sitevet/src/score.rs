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

/// The TS package's `test/runner.test.ts` cases.
#[cfg(test)]
mod tests {
    use super::*;

    fn f(status: Status, severity: Severity, category: &str) -> Finding {
        Finding {
            rule_id: "x".into(),
            category: category.into(),
            title: "x".into(),
            status,
            severity,
            message: String::new(),
            details: None,
            evidence: None,
            fix: None,
            key: String::new(),
        }
    }

    fn page(findings: Vec<Finding>) -> PageResult {
        PageResult {
            url: "u".into(),
            final_url: "u".into(),
            status: 200,
            error: None,
            depth: 0,
            redirect_chain: vec![],
            response_ms: 0,
            html_bytes: 0,
            truncated: false,
            score: score_findings(&findings),
            findings,
            metrics: vec![],
        }
    }

    #[test]
    fn all_error_passes_is_100_all_error_fails_is_0() {
        assert_eq!(
            score_findings(&[f(Status::Pass, Severity::Error, "c"), f(Status::Pass, Severity::Error, "c")]),
            100
        );
        assert_eq!(score_findings(&[f(Status::Fail, Severity::Error, "c"), f(Status::Fail, Severity::Error, "c")]), 0);
    }

    #[test]
    fn warn_counts_as_quarter_credit() {
        assert_eq!(score_findings(&[f(Status::Warn, Severity::Error, "c")]), 25);
    }

    #[test]
    fn info_and_empty_do_not_score() {
        assert_eq!(score_findings(&[f(Status::Info, Severity::Error, "c")]), 100);
        assert_eq!(score_findings(&[]), 100);
    }

    #[test]
    fn harsh_overall_penalty() {
        // One error-severity rule fails on 1 of 2 pages: penalty 22 * 0.5 = 11.
        let pages = [
            page(vec![f(Status::Pass, Severity::Error, "core-seo")]),
            page(vec![f(Status::Fail, Severity::Error, "security")]),
        ];
        assert_eq!(compute_overall(&pages), 89);
    }

    #[test]
    fn overall_drops_when_an_error_fails_everywhere() {
        let pages: Vec<PageResult> =
            (0..10).map(|_| page(vec![f(Status::Fail, Severity::Error, "security")])).collect();
        assert_eq!(compute_overall(&pages), 78);
    }

    #[test]
    fn warns_use_the_warn_factor_and_floor_at_zero() {
        // A warning-severity rule warning on every page: 11 * 0.55 = 6.05.
        let pages = [page(vec![f(Status::Warn, Severity::Warning, "c")])];
        assert_eq!(compute_overall(&pages), 94);
        let mut many = Vec::new();
        for i in 0..10 {
            let mut x = f(Status::Fail, Severity::Error, "c");
            x.rule_id = format!("r{i}");
            many.push(x);
        }
        assert_eq!(compute_overall(&[page(many)]), 0);
    }
}
