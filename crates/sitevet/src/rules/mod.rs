//! The rule and collector traits, and the registry. M2 fills the registry with the ported rules.

#[macro_use]
pub(crate) mod kit;
mod accessibility;
mod analytics;
mod content;
mod core_seo;
mod crawlability;
mod eeat;
mod images;
mod internationalization;
mod legal;
mod links;
#[cfg(test)]
mod parity;
mod performance;
mod security;
mod social_media;
mod structured_data;
#[cfg(test)]
pub(crate) mod testkit;
mod url_structure;

pub(crate) use internationalization::HREFLANG;
pub(crate) use links::GENERIC_ANCHOR;
pub(crate) use structured_data::JSON_LD;

use std::sync::Arc;

use scraper::Html;
use serde::{Deserialize, Serialize};

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

    pub fn pass(message: impl Into<String>) -> Self {
        Self::new(Status::Pass, message)
    }

    pub fn fail(message: impl Into<String>) -> Self {
        Self::new(Status::Fail, message)
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self::new(Status::Warn, message)
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(Status::Info, message)
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }
}

/// What the rendered stage measured on one page (M4 fills it).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RenderedData {
    /// False when the page could not be rendered.
    pub ok: bool,
    pub error: Option<String>,
    pub axe: Vec<AxeViolation>,
    pub axe_passes: Vec<AxePass>,
    /// None when the provider measured no performance numbers.
    pub metrics: Option<PerfMetrics>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AxeViolation {
    pub id: String,
    /// minor, moderate, serious or critical.
    pub impact: Option<String>,
    pub help: String,
    pub description: String,
    pub help_url: String,
    pub nodes: u32,
    pub sample: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AxePass {
    pub id: String,
    pub help: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PerfMetrics {
    pub ttfb_ms: f64,
    pub load_ms: f64,
    /// 0 when not measured.
    pub lcp_ms: f64,
    pub cls: f64,
    pub dom_nodes: u32,
    pub requests: u32,
    pub transfer_bytes: u64,
}

/// What a rule or collector can read.
#[non_exhaustive]
pub struct RuleCx<'a> {
    pub page: &'a PageData,
    /// Only in [`Stage::Dom`].
    pub dom: Option<&'a Html>,
    /// The raw HTML as decoded. Only in [`Stage::Dom`].
    pub html: Option<&'a str>,
    /// Browser or PageSpeed data. Only in [`Stage::Rendered`].
    pub rendered: Option<&'a RenderedData>,
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

    pub fn with_status(mut self, status: ValueStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Keeps the first [`ROW_CAP`] rows and records the full count.
    pub fn table(
        id: impl Into<String>,
        label: impl Into<String>,
        columns: &[&str],
        rows: Vec<Vec<MetricCell>>,
    ) -> Self {
        Self::table_capped(id, label, columns, rows, ROW_CAP)
    }

    /// [`MetricInput::table`] with its own row cap.
    pub fn table_capped(
        id: impl Into<String>,
        label: impl Into<String>,
        columns: &[&str],
        mut rows: Vec<Vec<MetricCell>>,
        cap: usize,
    ) -> Self {
        let total = rows.len() as u32;
        rows.truncate(cap);
        let body = MetricBody::Table { columns: columns.iter().map(|c| c.to_string()).collect(), rows, total };
        Self { id: id.into(), label: label.into(), status: None, note: None, body }
    }
}

/// A measurement. Its output never moves the score.
pub trait Collector: Send + Sync {
    fn meta(&self) -> CollectorMeta;
    fn collect(&self, cx: &RuleCx<'_>) -> Vec<MetricInput>;
}

/// Every built-in rule, in the TS package's order.
pub fn all_rules() -> Vec<Arc<dyn Rule>> {
    [
        core_seo::rules(),
        content::rules(),
        links::rules(),
        images::rules(),
        structured_data::rules(),
        security::rules(),
        crawlability::rules(),
        url_structure::rules(),
        social_media::rules(),
        internationalization::rules(),
        legal::rules(),
        analytics::rules(),
        eeat::rules(),
        accessibility::rules(),
        performance::rules(),
    ]
    .concat()
}

/// Every built-in collector, in the TS package's order.
pub fn all_collectors() -> Vec<Arc<dyn Collector>> {
    crate::metrics::all()
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

pub(crate) fn wanted(options: &Options, category: &str) -> bool {
    (options.only.is_empty() || options.only.iter().any(|c| c == category))
        && !options.skip.iter().any(|c| c == category)
}
