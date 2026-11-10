//! The report types. JSON field names are snake_case, enums are lowercase strings.

mod diff;
mod render;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use diff::{BaselineRef, CategoryChange, FindingRef, ReportDiff, diff};
pub use render::{Format, RenderOptions, render, render_with, report_schema};

use crate::Options;

/// A finding's verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Status {
    Pass,
    Fail,
    Warn,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A metric's own status. Metrics never touch the score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ValueStatus {
    Ok,
    Warn,
    Bad,
    Missing,
}

/// The result of one audit.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct AuditReport {
    pub schema_version: u32,
    pub sitevet_version: String,
    pub start_url: String,
    pub final_url: String,
    /// RFC 3339, UTC.
    pub generated_at: String,
    pub duration_ms: u64,
    /// The options used. `psi_key` is never written.
    pub options: Options,
    /// Overall score, 0 to 100.
    pub score: u8,
    /// Sorted by category.
    pub categories: Vec<CategoryScore>,
    /// In crawl order.
    pub pages: Vec<PageResult>,
    /// URLs skipped because robots.txt disallows them.
    pub blocked: Vec<BlockedUrl>,
    /// Rules that did not run, with the reason, e.g. "needs browser or psi_key".
    #[serde(default)]
    pub skipped_rules: Vec<SkippedRule>,
    /// Changes against a baseline, when one was given.
    #[serde(default)]
    pub changes: Option<ReportDiff>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct SkippedRule {
    pub rule_id: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct CategoryScore {
    pub category: String,
    pub score: u8,
    pub pass: u32,
    pub fail: u32,
    pub warn: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct PageResult {
    /// Normalised, without the fragment.
    pub url: String,
    pub final_url: String,
    /// 0 when the fetch failed.
    pub status: u16,
    pub error: Option<String>,
    /// Click depth from the start page. Pages seeded from the sitemap count as depth 1.
    pub depth: u32,
    /// Each URL that answered with a redirect, in order.
    pub redirect_chain: Vec<String>,
    pub response_ms: u64,
    pub html_bytes: u64,
    /// True when the HTML went over 10 MB and was cut.
    pub truncated: bool,
    pub score: u8,
    pub findings: Vec<Finding>,
    pub metrics: Vec<Metric>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct Finding {
    pub rule_id: String,
    pub category: String,
    pub title: String,
    pub status: Status,
    pub severity: Severity,
    pub message: String,
    pub details: Option<String>,
    /// Offending markup, header or value, at most 2,000 characters.
    pub evidence: Option<String>,
    pub fix: Option<String>,
    /// Stable identity for diffs: a hash of rule ID, page URL and evidence.
    pub key: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct Metric {
    pub id: String,
    pub category: String,
    pub label: String,
    pub status: Option<ValueStatus>,
    pub note: Option<String>,
    pub body: MetricBody,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[non_exhaustive]
pub enum MetricBody {
    Value {
        value: Option<MetricCell>,
        unit: Option<String>,
    },
    /// `rows` holds at most [`crate::ROW_CAP`] rows. `total` is the count before the cap.
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<MetricCell>>,
        total: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MetricCell {
    Number(f64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub struct BlockedUrl {
    pub url: String,
    /// The robots.txt `Disallow` line that matched, when one can be pinned down.
    pub rule: Option<String>,
}
