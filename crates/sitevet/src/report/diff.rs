//! Comparing two reports by stable finding keys.

use std::collections::{BTreeMap, HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{AuditReport, Status};

/// What changed between a baseline report and a newer one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct ReportDiff {
    pub baseline: BaselineRef,
    /// Overall score, newer minus baseline.
    pub score_change: i16,
    /// Every category in either report, sorted by name.
    pub categories: Vec<CategoryChange>,
    /// Findings failing or warning now that passed or did not exist in the baseline.
    pub new_failures: Vec<FindingRef>,
    /// Findings failing or warning in the baseline that pass or are gone now.
    pub fixed: Vec<FindingRef>,
    pub pages_added: Vec<String>,
    pub pages_removed: Vec<String>,
}

/// Which report the comparison is against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct BaselineRef {
    /// History run ID, when the baseline came from history.
    pub run_id: Option<i64>,
    /// File path, when the baseline came from a JSON report file.
    pub file: Option<String>,
    /// When the baseline report was made.
    pub generated_at: String,
    pub score: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct CategoryChange {
    pub category: String,
    /// None when the category did not run.
    pub before: Option<u8>,
    pub after: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct FindingRef {
    pub key: String,
    pub rule_id: String,
    pub url: String,
    pub title: String,
    pub status: Status,
    pub message: String,
}

/// Compares `baseline` with `current`. Findings match on their `key` (rule, page and evidence),
/// so a finding whose evidence changed counts as fixed plus new.
pub fn diff(baseline: &AuditReport, current: &AuditReport) -> ReportDiff {
    let failing = |r: &AuditReport| -> HashMap<String, FindingRef> {
        r.pages
            .iter()
            .flat_map(|p| p.findings.iter().map(move |f| (p, f)))
            .filter(|(_, f)| matches!(f.status, Status::Fail | Status::Warn))
            .map(|(p, f)| {
                let r = FindingRef {
                    key: f.key.clone(),
                    rule_id: f.rule_id.clone(),
                    url: p.url.clone(),
                    title: f.title.clone(),
                    status: f.status,
                    message: f.message.clone(),
                };
                (f.key.clone(), r)
            })
            .collect()
    };
    let (before, after) = (failing(baseline), failing(current));
    let sorted = |mut v: Vec<FindingRef>| {
        v.sort_by(|a, b| (&a.url, &a.rule_id, &a.key).cmp(&(&b.url, &b.rule_id, &b.key)));
        v
    };
    let new_failures = sorted(after.iter().filter(|(k, _)| !before.contains_key(*k)).map(|(_, f)| f.clone()).collect());
    let fixed = sorted(before.iter().filter(|(k, _)| !after.contains_key(*k)).map(|(_, f)| f.clone()).collect());

    let mut categories: BTreeMap<&str, CategoryChange> = BTreeMap::new();
    for c in &baseline.categories {
        categories
            .insert(&c.category, CategoryChange { category: c.category.clone(), before: Some(c.score), after: None });
    }
    for c in &current.categories {
        categories
            .entry(&c.category)
            .or_insert_with(|| CategoryChange { category: c.category.clone(), before: None, after: None })
            .after = Some(c.score);
    }

    let urls = |r: &AuditReport| r.pages.iter().map(|p| p.url.clone()).collect::<HashSet<_>>();
    let (old_urls, new_urls) = (urls(baseline), urls(current));
    let mut pages_added: Vec<String> = new_urls.difference(&old_urls).cloned().collect();
    let mut pages_removed: Vec<String> = old_urls.difference(&new_urls).cloned().collect();
    pages_added.sort();
    pages_removed.sort();

    ReportDiff {
        baseline: BaselineRef {
            run_id: None,
            file: None,
            generated_at: baseline.generated_at.clone(),
            score: baseline.score,
        },
        score_change: i16::from(current.score) - i16::from(baseline.score),
        categories: categories.into_values().collect(),
        new_failures,
        fixed,
        pages_added,
        pages_removed,
    }
}
