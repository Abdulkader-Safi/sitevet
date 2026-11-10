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

mod crawl;
mod engine;
mod error;
mod extract;
mod options;
mod report;
mod rules;
mod score;
mod time;

pub use crawl::{LinkStatus, LinkStatusMap, RobotsInfo, SiteData, SitemapInfo};
pub use engine::{AuditEvent, AuditHandle, AuditStream, audit, audit_stream, spawn};
pub use error::{Error, Result};
pub use extract::{Heading, Image, Link, MetaTag, PageData, Script};
pub use options::Options;
pub use report::{
    AuditReport, BlockedUrl, CategoryScore, Finding, Metric, MetricBody, MetricCell, PageResult, Severity, Status,
    ValueStatus,
};
pub use rules::{
    Collector, CollectorMeta, MetricInput, ROW_CAP, Rule, RuleCx, RuleMeta, RuleResult, Stage, all_collectors,
    all_rules, select_rules,
};
pub use score::{compute_overall, score_findings};

/// The HTML parser rules see in the [`Stage::Dom`] stage.
pub use scraper;
