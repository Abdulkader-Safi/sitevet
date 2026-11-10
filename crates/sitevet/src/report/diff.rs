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
    /// Failing findings left out of `fixed` and `new_failures` because their check ran on that
    /// page in only one of the two reports (say, a `--browser` run against one without).
    #[serde(default)]
    pub not_compared: usize,
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
/// so a finding whose evidence changed counts as fixed plus new. A finding on a page both
/// reports audited counts only when its rule has a result on that page in both; otherwise the
/// check did not run on one side and the finding goes to `not_compared`.
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
    let checked = |r: &AuditReport| -> HashSet<(String, String)> {
        r.pages.iter().flat_map(|p| p.findings.iter().map(|f| (p.url.clone(), f.rule_id.clone()))).collect()
    };
    let urls = |r: &AuditReport| r.pages.iter().map(|p| p.url.clone()).collect::<HashSet<_>>();
    let (old_urls, new_urls) = (urls(baseline), urls(current));
    let (old_checked, new_checked) = (checked(baseline), checked(current));
    // An added or removed page keeps counting: its findings are new or fixed.
    let comparable = |f: &FindingRef, other_urls: &HashSet<String>, other: &HashSet<(String, String)>| {
        !other_urls.contains(&f.url) || other.contains(&(f.url.clone(), f.rule_id.clone()))
    };
    let not_compared = before.values().filter(|f| !comparable(f, &new_urls, &new_checked)).count()
        + after.values().filter(|f| !comparable(f, &old_urls, &old_checked)).count();
    let sorted = |mut v: Vec<FindingRef>| {
        v.sort_by(|a, b| (&a.url, &a.rule_id, &a.key).cmp(&(&b.url, &b.rule_id, &b.key)));
        v
    };
    let new_failures = sorted(
        after
            .iter()
            .filter(|(k, f)| !before.contains_key(*k) && comparable(f, &old_urls, &old_checked))
            .map(|(_, f)| f.clone())
            .collect(),
    );
    let fixed = sorted(
        before
            .iter()
            .filter(|(k, f)| !after.contains_key(*k) && comparable(f, &new_urls, &new_checked))
            .map(|(_, f)| f.clone())
            .collect(),
    );

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
        not_compared,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One page at `/` with (rule_id, status, key) findings.
    fn report(findings: &[(&str, &str, &str)]) -> AuditReport {
        let findings: Vec<String> = findings
            .iter()
            .map(|(rule, status, key)| {
                format!(
                    r#"{{"rule_id":"{rule}","category":"x","title":"t","status":"{status}","severity":"warning",
                        "message":"m","details":null,"evidence":null,"fix":null,"key":"{key}"}}"#
                )
            })
            .collect();
        let json = format!(
            r#"{{"schema_version":1,"sitevet_version":"0","start_url":"https://a.test/","final_url":"https://a.test/",
                "generated_at":"2026-11-10T09:30:00Z","duration_ms":1,"options":{{}},"score":80,"categories":[],"blocked":[],
                "pages":[{{"url":"https://a.test/","final_url":"https://a.test/","status":200,"error":null,"depth":0,
                "redirect_chain":[],"response_ms":1,"html_bytes":1,"truncated":false,"score":80,
                "findings":[{}],"metrics":[]}}]}}"#,
            findings.join(",")
        );
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn checks_that_ran_on_one_side_are_not_compared() {
        // Run 1 had the browser: an axe failure. Run 2 did not, so axe has no result there.
        let with_browser = report(&[("core-seo/title", "fail", "t1"), ("accessibility/region", "fail", "a1")]);
        let without = report(&[("core-seo/title", "pass", "t2")]);
        let d = diff(&with_browser, &without);
        assert_eq!(d.fixed.iter().map(|f| f.key.as_str()).collect::<Vec<_>>(), ["t1"]);
        assert!(d.new_failures.is_empty());
        assert_eq!(d.not_compared, 1);
        // The other way round, the axe failure is not "new" either.
        let d = diff(&without, &with_browser);
        assert_eq!(d.new_failures.iter().map(|f| f.key.as_str()).collect::<Vec<_>>(), ["t1"]);
        assert_eq!(d.not_compared, 1);
    }
}
