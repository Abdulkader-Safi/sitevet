//! sitevet: a website audit engine.
//!
//! Crawl a site, run rules and collectors over every page, score the result and get a typed
//! [`AuditReport`] back. Three entry points share one engine:
//!
//! - [`audit`]: an async call inside a tokio context.
//! - [`audit_stream`]: the same audit as a [`futures::Stream`] of [`AuditEvent`]s.
//! - [`spawn`]: runs on a runtime sitevet owns, so apps on any executor can drive it.
//!
//! ```no_run
//! # async fn demo() -> sitevet::Result<()> {
//! let report = sitevet::audit("https://example.com", sitevet::Options::default()).await?;
//! println!("{} pages, score {}", report.pages.len(), report.score);
//! # Ok(())
//! # }
//! ```

// Rules first: their helper macros are used by the collectors.
#[macro_use]
mod rules;
mod crawl;
mod engine;
mod error;
mod extract;
#[cfg(feature = "history")]
mod history;
mod metrics;
mod options;
mod render_stage;
mod report;
mod score;
mod text;
mod time;

pub use crawl::{LinkStatus, LinkStatusMap, RobotsInfo, SiteData, SitemapInfo};
pub use engine::{
    AuditEvent, AuditHandle, AuditStream, audit, audit_score, audit_stream, audit_to_html, audit_to_markdown, spawn,
};
pub use error::{Error, Result};
pub use extract::{Heading, Image, Link, MetaTag, PageData, Script};
#[cfg(feature = "history")]
pub use history::{History, RunId, RunSummary};
pub use options::Options;
#[doc(hidden)] // shared with the CLI for its own config keys
pub use options::unknown_key_reason;
pub use report::{
    AuditReport, BaselineRef, BlockedUrl, CategoryChange, CategoryScore, Finding, FindingRef, Format, Metric,
    MetricBody, MetricCell, PageResult, RenderOptions, ReportDiff, Severity, SkippedRule, Status, ValueStatus, diff,
    render, render_with, report_schema,
};
pub use rules::{
    AxePass, AxeViolation, Collector, CollectorMeta, MetricInput, PerfMetrics, ROW_CAP, RenderedData, Rule, RuleCx,
    RuleMeta, RuleResult, Stage, all_collectors, all_rules, select_rules,
};
pub use score::{compute_overall, score_findings};

/// The HTML parser rules see in the [`Stage::Dom`] stage.
pub use scraper;
