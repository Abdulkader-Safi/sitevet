//! The rule and collector traits, and the registry. M2 fills the registry with the ported rules.

use std::sync::Arc;

use scraper::Html;

use crate::report::{MetricBody, MetricCell, Severity, Status, ValueStatus};
use crate::{LinkStatusMap, Options, PageData, SiteData};

/// Tables keep at most this many rows (the TS package's `ROW_CAP`).
pub const ROW_CAP: usize = 25;

/// When a rule or collector runs, by the inputs it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Stage {
    /// During the crawl, with the parsed DOM.
    Dom,
    /// After the crawl, once the page's links have been checked.
    Enriched,
    /// After the rendered stage (browser or PageSpeed Insights).
    Rendered,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RuleMeta {
    pub id: &'static str,
    pub category: &'static str,
    pub title: &'static str,
    pub severity: Severity,
    pub stage: Stage,
    pub fix: Option<&'static str>,
}

impl RuleMeta {
    pub fn new(
        id: &'static str,
        category: &'static str,
        title: &'static str,
        severity: Severity,
        stage: Stage,
    ) -> Self {
        Self { id, category, title, severity, stage, fix: None }
    }
}

/// What a rule returns. Unset overrides fall back to the rule's [`RuleMeta`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RuleResult {
    pub status: Status,
    pub message: String,
    pub details: Option<String>,
    pub evidence: Option<String>,
    pub rule_id: Option<String>,
    pub title: Option<String>,
    pub fix: Option<String>,
    pub severity: Option<Severity>,
}

impl RuleResult {
    pub fn new(status: Status, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            details: None,
            evidence: None,
            rule_id: None,
            title: None,
            fix: None,
            severity: None,
        }
    }
}

/// What a rule or collector can read.
#[non_exhaustive]
pub struct RuleCx<'a> {
    pub page: &'a PageData,
    /// Only in [`Stage::Dom`].
    pub dom: Option<&'a Html>,
    /// Status of every link on the page, from [`Stage::Enriched`] on. Empty before that.
    pub links: &'a LinkStatusMap,
    pub site: &'a SiteData,
}

/// One check. Rules are synchronous: every network result they need is fetched before their stage.
pub trait Rule: Send + Sync {
    fn meta(&self) -> RuleMeta;
    fn run(&self, cx: &RuleCx<'_>) -> Vec<RuleResult>;
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CollectorMeta {
    pub id: &'static str,
    pub category: &'static str,
    pub stage: Stage,
}

impl CollectorMeta {
    pub fn new(id: &'static str, category: &'static str, stage: Stage) -> Self {
        Self { id, category, stage }
    }
}

/// A metric before the engine stamps the collector's category on it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MetricInput {
    pub id: String,
    pub label: String,
    pub status: Option<ValueStatus>,
    pub note: Option<String>,
    pub body: MetricBody,
}

impl MetricInput {
    pub fn value(
        id: impl Into<String>,
        label: impl Into<String>,
        value: Option<MetricCell>,
        unit: Option<&str>,
    ) -> Self {
        let body = MetricBody::Value { value, unit: unit.map(str::to_string) };
        Self { id: id.into(), label: label.into(), status: None, note: None, body }
    }

    /// Keeps the first [`ROW_CAP`] rows and records the full count.
    pub fn table(
        id: impl Into<String>,
        label: impl Into<String>,
        columns: &[&str],
        mut rows: Vec<Vec<MetricCell>>,
    ) -> Self {
        let total = rows.len() as u32;
        rows.truncate(ROW_CAP);
        let body = MetricBody::Table { columns: columns.iter().map(|c| c.to_string()).collect(), rows, total };
        Self { id: id.into(), label: label.into(), status: None, note: None, body }
    }
}

/// A measurement. Its output never moves the score.
pub trait Collector: Send + Sync {
    fn meta(&self) -> CollectorMeta;
    fn collect(&self, cx: &RuleCx<'_>) -> Vec<MetricInput>;
}

/// Every built-in rule.
pub fn all_rules() -> Vec<Arc<dyn Rule>> {
    Vec::new()
}

/// Every built-in collector.
pub fn all_collectors() -> Vec<Arc<dyn Collector>> {
    Vec::new()
}

/// The rules an audit with these options runs: `options.rules` or the registry, filtered by
/// `only` and `skip`.
pub fn select_rules(options: &Options) -> Vec<Arc<dyn Rule>> {
    let rules = options.rules.clone().unwrap_or_else(all_rules);
    rules.into_iter().filter(|r| wanted(options, r.meta().category)).collect()
}

pub(crate) fn select_collectors(options: &Options) -> Vec<Arc<dyn Collector>> {
    let collectors = options.collectors.clone().unwrap_or_else(all_collectors);
    collectors.into_iter().filter(|c| wanted(options, c.meta().category)).collect()
}

fn wanted(options: &Options, category: &str) -> bool {
    (options.only.is_empty() || options.only.iter().any(|c| c == category))
        && !options.skip.iter().any(|c| c == category)
}
