use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::rules::{Collector, Rule};
use crate::{Error, Result};

/// The most pages one audit may crawl (NFR-05).
pub const MAX_PAGES_CAP: u32 = 10_000;

/// Every audit setting. `sitevet.toml` keys and CLI flags map one to one onto these fields.
///
/// The struct is `#[non_exhaustive]`, so build it from [`Options::default`] and set fields:
///
/// ```
/// let mut options = sitevet::Options::default();
/// options.max_pages = 20;
/// assert!(options.validate().is_ok());
/// ```
#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct Options {
    /// Categories to run. Empty means all.
    pub only: Vec<String>,
    /// Categories to leave out.
    pub skip: Vec<String>,
    /// Default 100, at most 10,000.
    pub max_pages: u32,
    /// Click depth from the start page. Default 5.
    pub max_depth: u32,
    /// Requests in flight per host. Default 5.
    pub concurrency: u32,
    /// Gap between request starts to one host. Default 0.
    pub delay_ms: u64,
    /// Per request. Default 15,000.
    pub timeout_ms: u64,
    /// Default `sitevet/<version> (+https://github.com/Abdulkader-Safi/sitevet)`.
    pub user_agent: Option<String>,
    /// Default false: robots.txt is obeyed.
    pub ignore_robots: bool,
    /// Seed the crawl from the sitemap too. Default true.
    pub crawl_sitemap: bool,
    /// Allow private, loopback and link-local addresses. Unset means allowed only when the
    /// start URL is itself private.
    pub allow_private_network: Option<bool>,
    /// Rendered checks in a local Chrome. Needs the `browser` feature. Default false.
    pub browser: bool,
    /// Default: auto-detect, or `SITEVET_CHROME`.
    pub chrome_path: Option<PathBuf>,
    /// Find links after JavaScript runs. Needs `browser`. Default false.
    pub render_crawl: bool,
    /// PageSpeed Insights key, or `SITEVET_PSI_KEY`. Never serialised.
    #[serde(skip_serializing)]
    pub psi_key: Option<String>,
    /// Default 5.
    pub psi_max_pages: u32,
    /// Image size probes. Needs the `media` feature. Default false.
    pub media: bool,
    /// Default 200.
    pub media_max_requests: u32,
    /// Needs `browser`. Default false.
    pub screenshots: bool,
    /// Default: next to the report, or the current folder.
    pub screenshot_dir: Option<PathBuf>,
    /// Default 3.
    pub click_depth_warn: u32,
    /// Default 300.
    pub slow_resource_ms: u64,
    /// Worker threads of the runtime [`crate::spawn`] creates. Default 2.
    pub runtime_threads: usize,
    /// Replaces the built-in rules. Library only.
    #[serde(skip)]
    pub rules: Option<Vec<Arc<dyn Rule>>>,
    /// Replaces the built-in collectors. Library only.
    #[serde(skip)]
    pub collectors: Option<Vec<Arc<dyn Collector>>>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            only: Vec::new(),
            skip: Vec::new(),
            max_pages: 100,
            max_depth: 5,
            concurrency: 5,
            delay_ms: 0,
            timeout_ms: 15_000,
            user_agent: None,
            ignore_robots: false,
            crawl_sitemap: true,
            allow_private_network: None,
            browser: false,
            chrome_path: None,
            render_crawl: false,
            psi_key: None,
            psi_max_pages: 5,
            media: false,
            media_max_requests: 200,
            screenshots: false,
            screenshot_dir: None,
            click_depth_warn: 3,
            slow_resource_ms: 300,
            runtime_threads: 2,
            rules: None,
            collectors: None,
        }
    }
}

// Hand-written so psi_key can never reach a log line.
impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "Options {json}")
    }
}

impl Options {
    /// Reads `sitevet.toml` text. A `[cli]` table is ignored here: it belongs to the CLI.
    /// Unknown keys are errors.
    pub fn from_toml_str(text: &str) -> Result<Self> {
        let toml_err = |e: toml::de::Error| Error::InvalidOptions {
            field: "sitevet.toml".into(),
            reason: e.message().to_string(),
        };
        let mut table: toml::Table = text.parse().map_err(toml_err)?;
        table.remove("cli");
        if let Some(key) = table.keys().find(|k| !Self::KEYS.contains(&k.as_str())) {
            return Err(Error::InvalidOptions { field: key.clone(), reason: unknown_key_reason(key, Self::KEYS) });
        }
        toml::Value::Table(table).try_into().map_err(toml_err)
    }

    /// Every `sitevet.toml` key, in field order.
    pub const KEYS: &'static [&'static str] = &[
        "only",
        "skip",
        "max_pages",
        "max_depth",
        "concurrency",
        "delay_ms",
        "timeout_ms",
        "user_agent",
        "ignore_robots",
        "crawl_sitemap",
        "allow_private_network",
        "browser",
        "chrome_path",
        "render_crawl",
        "psi_key",
        "psi_max_pages",
        "media",
        "media_max_requests",
        "screenshots",
        "screenshot_dir",
        "click_depth_warn",
        "slow_resource_ms",
        "runtime_threads",
    ];

    /// Checks limits and feature requirements.
    pub fn validate(&self) -> Result<()> {
        let bad = |field: &str, reason: String| Err(Error::InvalidOptions { field: field.into(), reason });
        if self.max_pages == 0 || self.max_pages > MAX_PAGES_CAP {
            return bad("max_pages", format!("must be from 1 to {MAX_PAGES_CAP} (the cap), got {}", self.max_pages));
        }
        if self.concurrency == 0 {
            return bad("concurrency", "must be at least 1".into());
        }
        if self.timeout_ms == 0 {
            return bad("timeout_ms", "must be at least 1".into());
        }
        if self.runtime_threads == 0 {
            return bad("runtime_threads", "must be at least 1".into());
        }
        if self.browser && !cfg!(feature = "browser") {
            return bad("browser", "needs sitevet built with the `browser` feature".into());
        }
        if self.media && !cfg!(feature = "media") {
            return bad("media", "needs sitevet built with the `media` feature".into());
        }
        for (field, on) in [("render_crawl", self.render_crawl), ("screenshots", self.screenshots)] {
            if on && !self.browser {
                return bad(field, "needs `browser = true`".into());
            }
        }
        Ok(())
    }

    pub(crate) fn user_agent(&self) -> String {
        self.user_agent.clone().unwrap_or_else(|| {
            format!("sitevet/{} (+https://github.com/Abdulkader-Safi/sitevet)", env!("CARGO_PKG_VERSION"))
        })
    }
}

/// "unknown key, did you mean `x`?" with the closest valid key, when one is close.
pub fn unknown_key_reason(key: &str, valid: &[&str]) -> String {
    let closest = valid.iter().map(|v| (edit_distance(key, v), *v)).min();
    match closest {
        Some((d, v)) if d <= 3.max(key.len() / 3) => format!("unknown key, did you mean `{v}`?"),
        _ => "unknown key".to_string(),
    }
}

/// Levenshtein distance.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = if ca == *cb { prev } else { 1 + prev.min(row[j]).min(row[j + 1]) };
            prev = cur;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let o = Options::default();
        assert_eq!((o.max_pages, o.max_depth, o.concurrency, o.delay_ms, o.timeout_ms), (100, 5, 5, 0, 15_000));
        assert!(!o.ignore_robots && o.crawl_sitemap && !o.browser && !o.media && !o.screenshots);
        assert_eq!((o.psi_max_pages, o.media_max_requests, o.runtime_threads), (5, 200, 2));
        assert!(o.validate().is_ok());
        assert!(o.user_agent().starts_with("sitevet/"));
    }

    #[test]
    fn max_pages_over_the_cap_names_the_cap() {
        let mut o = Options::default();
        o.max_pages = 10_001;
        let err = o.validate().unwrap_err().to_string();
        assert!(err.contains("max_pages") && err.contains("10000"), "{err}");
        o.max_pages = 10_000;
        assert!(o.validate().is_ok());
    }

    #[test]
    fn screenshots_need_browser() {
        let mut o = Options::default();
        o.screenshots = true;
        assert!(o.validate().unwrap_err().to_string().contains("screenshots"));
    }

    #[test]
    fn toml_sets_every_field_and_skips_the_cli_table() {
        let text = r#"
            only = ["security"]
            skip = ["links"]
            max_pages = 7
            max_depth = 2
            concurrency = 3
            delay_ms = 250
            timeout_ms = 900
            user_agent = "bot/1"
            ignore_robots = true
            crawl_sitemap = false
            allow_private_network = true
            browser = true
            chrome_path = "/opt/chrome"
            render_crawl = true
            psi_key = "k"
            psi_max_pages = 9
            media = true
            media_max_requests = 50
            screenshots = true
            screenshot_dir = "shots"
            click_depth_warn = 4
            slow_resource_ms = 500
            runtime_threads = 4

            [cli]
            format = "json"
        "#;
        let o = Options::from_toml_str(text).unwrap();
        assert_eq!(o.only, ["security"]);
        assert_eq!((o.max_pages, o.delay_ms, o.runtime_threads), (7, 250, 4));
        assert_eq!(o.psi_key.as_deref(), Some("k"));
        assert_eq!(o.allow_private_network, Some(true));
        // psi_key is read but never written back out.
        let back = toml::to_string(&o).unwrap();
        assert!(!back.contains("psi_key") && back.contains("max_pages = 7"));
        assert!(!format!("{o:?}").contains("\"k\""));
    }

    #[test]
    fn unknown_toml_key_is_named_with_the_closest_key() {
        let err = Options::from_toml_str("max_page = 3").unwrap_err().to_string();
        assert_eq!(err, "invalid option `max_page`: unknown key, did you mean `max_pages`?");
        let err = Options::from_toml_str("zzz = 3").unwrap_err().to_string();
        assert_eq!(err, "invalid option `zzz`: unknown key");
    }

    #[test]
    fn keys_cover_every_field() {
        let mut o = Options::default();
        o.user_agent = Some("x".into());
        o.allow_private_network = Some(true);
        o.chrome_path = Some("c".into());
        o.screenshot_dir = Some("s".into());
        let serde_json::Value::Object(map) = serde_json::to_value(&o).unwrap() else { panic!() };
        let mut serialised: Vec<&str> = map.keys().map(String::as_str).collect();
        serialised.push("psi_key"); // read, never written
        serialised.sort();
        let mut keys = Options::KEYS.to_vec();
        keys.sort();
        assert_eq!(serialised, keys);
    }
}
