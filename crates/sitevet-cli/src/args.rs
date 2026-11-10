//! Command-line flags. Every `Options` field has a flag (kebab-case of its `sitevet.toml` key).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "sitevet", version, about = "Audit a website for SEO, security, accessibility and performance.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
    /// No colour, no progress styling.
    #[arg(long, global = true)]
    pub plain: bool,
    /// Debug logs on stderr. SITEVET_LOG sets a custom filter.
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Audit a site and write a report.
    Audit(Box<AuditArgs>),
    /// List every rule.
    Rules(RulesArgs),
    /// Write a sitevet.toml with every setting at its default.
    Init(InitArgs),
    /// List saved runs, newest first, or delete and prune them.
    History(HistoryArgs),
    /// Render a saved run without crawling again.
    Show(ShowArgs),
    /// Compare two saved runs.
    Diff(DiffArgs),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Json,
    Md,
    Html,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailOn {
    /// Any failing error-severity finding.
    Error,
    /// Any failing or warning finding of error or warning severity.
    Warning,
}

#[derive(Args, Default)]
pub struct AuditArgs {
    /// The start URL, e.g. https://example.com
    pub url: String,
    /// Report format. Default: from the --out extension, else html.
    #[arg(long, value_enum)]
    pub format: Option<ReportFormat>,
    /// Write the report here instead of stdout.
    #[arg(long, short)]
    pub out: Option<PathBuf>,
    /// Config file. Default: ./sitevet.toml when it exists.
    #[arg(long)]
    pub config: Option<PathBuf>,
    /// Exit 1 when the overall score is below this.
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub fail_under: Option<u8>,
    /// Exit 1 when a finding of this severity fails.
    #[arg(long, value_enum)]
    pub fail_on: Option<FailOn>,
    /// Print only errors.
    #[arg(long, short)]
    pub quiet: bool,
    /// Do not save this run to history.
    #[arg(long)]
    pub no_save: bool,
    /// Compare with a saved run (its ID or "last" for this site) or a JSON report file.
    #[arg(long)]
    pub baseline: Option<String>,

    /// Categories to run, comma separated. Default: all.
    #[arg(long, value_delimiter = ',', help_heading = "Audit options")]
    pub only: Option<Vec<String>>,
    /// Categories to skip, comma separated.
    #[arg(long, value_delimiter = ',', help_heading = "Audit options")]
    pub skip: Option<Vec<String>>,
    /// Default 100, at most 10000.
    #[arg(long, help_heading = "Audit options")]
    pub max_pages: Option<u32>,
    /// Click depth from the start page. Default 5.
    #[arg(long, help_heading = "Audit options")]
    pub max_depth: Option<u32>,
    /// Requests in flight per host. Default 5.
    #[arg(long, help_heading = "Audit options")]
    pub concurrency: Option<u32>,
    /// Gap between requests to one host, in ms. Default 0.
    #[arg(long, help_heading = "Audit options")]
    pub delay_ms: Option<u64>,
    /// Per request, in ms. Default 15000.
    #[arg(long, help_heading = "Audit options")]
    pub timeout_ms: Option<u64>,
    /// Default: sitevet/<version> (+https://github.com/Abdulkader-Safi/sitevet)
    #[arg(long, help_heading = "Audit options")]
    pub user_agent: Option<String>,
    /// Do not obey robots.txt.
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true", help_heading = "Audit options")]
    pub ignore_robots: Option<bool>,
    /// Seed the crawl from the sitemap. Default true.
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true", help_heading = "Audit options")]
    pub crawl_sitemap: Option<bool>,
    /// Allow private and loopback addresses. Default: only when the start URL is private.
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true", help_heading = "Audit options")]
    pub allow_private_network: Option<bool>,
    /// Rendered checks in a local Chrome.
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true", help_heading = "Audit options")]
    pub browser: Option<bool>,
    /// Chrome or Chromium binary. Default: auto-detect.
    #[arg(long, env = "SITEVET_CHROME", help_heading = "Audit options")]
    pub chrome_path: Option<PathBuf>,
    /// Find links after JavaScript runs. Needs --browser.
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true", help_heading = "Audit options")]
    pub render_crawl: Option<bool>,
    /// PageSpeed Insights key. Prefer the environment variable.
    #[arg(long, env = "SITEVET_PSI_KEY", hide_env_values = true, help_heading = "Audit options")]
    pub psi_key: Option<String>,
    /// Pages sent to PageSpeed Insights. Default 5.
    #[arg(long, help_heading = "Audit options")]
    pub psi_max_pages: Option<u32>,
    /// Image size probes.
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true", help_heading = "Audit options")]
    pub media: Option<bool>,
    /// Default 200.
    #[arg(long, help_heading = "Audit options")]
    pub media_max_requests: Option<u32>,
    /// Page screenshots. Needs --browser.
    #[arg(long, num_args = 0..=1, require_equals = true, default_missing_value = "true", help_heading = "Audit options")]
    pub screenshots: Option<bool>,
    /// Default: next to the report, or the current folder.
    #[arg(long, help_heading = "Audit options")]
    pub screenshot_dir: Option<PathBuf>,
    /// Default 3.
    #[arg(long, help_heading = "Audit options")]
    pub click_depth_warn: Option<u32>,
    /// Default 300.
    #[arg(long, help_heading = "Audit options")]
    pub slow_resource_ms: Option<u64>,
    /// Worker threads. Default 2.
    #[arg(long, help_heading = "Audit options")]
    pub runtime_threads: Option<usize>,
}

impl AuditArgs {
    /// Flags win over the file: every flag given replaces its field.
    pub fn apply(&self, o: &mut sitevet::Options) {
        macro_rules! set {
            ($($field:ident),*) => { $( if let Some(v) = &self.$field { o.$field = v.clone(); } )* };
        }
        macro_rules! set_some {
            ($($field:ident),*) => { $( if let Some(v) = &self.$field { o.$field = Some(v.clone()); } )* };
        }
        set!(
            only,
            skip,
            max_pages,
            max_depth,
            concurrency,
            delay_ms,
            timeout_ms,
            ignore_robots,
            crawl_sitemap,
            browser
        );
        set!(render_crawl, psi_max_pages, media, media_max_requests, screenshots, click_depth_warn, slow_resource_ms);
        set!(runtime_threads);
        set_some!(user_agent, allow_private_network, chrome_path, psi_key, screenshot_dir);
    }
}

#[derive(Args)]
pub struct RulesArgs {
    /// Only this category.
    #[arg(long)]
    pub category: Option<String>,
    #[arg(long, value_enum, default_value = "table")]
    pub format: ListFormat,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ListFormat {
    Table,
    Md,
    Json,
}

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct HistoryArgs {
    #[command(subcommand)]
    pub action: Option<HistoryAction>,
    /// Only runs for this site (any URL on it).
    pub url: Option<String>,
    /// How many runs to list.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Subcommand)]
pub enum HistoryAction {
    /// Delete one run.
    Delete {
        id: i64,
        /// Do not ask for confirmation.
        #[arg(long, short)]
        yes: bool,
    },
    /// Keep the newest runs and delete the rest.
    Prune {
        #[arg(long)]
        keep: usize,
        /// Do not ask for confirmation.
        #[arg(long, short)]
        yes: bool,
    },
}

#[derive(Args)]
pub struct ShowArgs {
    pub id: i64,
    /// Default: from the --out extension, else html.
    #[arg(long, value_enum)]
    pub format: Option<ReportFormat>,
    #[arg(long, short)]
    pub out: Option<PathBuf>,
}

#[derive(Args)]
pub struct DiffArgs {
    /// The older run.
    pub a: i64,
    /// The newer run.
    pub b: i64,
    #[arg(long, value_enum, default_value = "text")]
    pub format: DiffFormat,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum DiffFormat {
    Text,
    Md,
    Json,
}

#[derive(Args)]
pub struct InitArgs {
    /// Overwrite an existing sitevet.toml.
    #[arg(long)]
    pub force: bool,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    /// FR-API-04: every field set through TOML and through flags gives equal Options.
    #[test]
    fn flags_and_toml_set_the_same_options() {
        let toml = r#"
            only = ["security", "links"]
            skip = ["legal"]
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
        "#;
        let (from_toml, _) = crate::config::parse(toml).unwrap();
        let cli = Cli::try_parse_from([
            "sitevet",
            "audit",
            "https://example.com",
            "--only",
            "security,links",
            "--skip",
            "legal",
            "--max-pages",
            "7",
            "--max-depth",
            "2",
            "--concurrency",
            "3",
            "--delay-ms",
            "250",
            "--timeout-ms",
            "900",
            "--user-agent",
            "bot/1",
            "--ignore-robots",
            "--crawl-sitemap=false",
            "--allow-private-network",
            "--browser",
            "--chrome-path",
            "/opt/chrome",
            "--render-crawl",
            "--psi-key",
            "k",
            "--psi-max-pages",
            "9",
            "--media",
            "--media-max-requests",
            "50",
            "--screenshots",
            "--screenshot-dir",
            "shots",
            "--click-depth-warn",
            "4",
            "--slow-resource-ms",
            "500",
            "--runtime-threads",
            "4",
        ])
        .unwrap();
        let Command::Audit(args) = cli.command else { panic!("not audit") };
        let mut from_flags = sitevet::Options::default();
        args.apply(&mut from_flags);
        let json = |o: &sitevet::Options| serde_json::to_value(o).unwrap();
        assert_eq!(json(&from_flags), json(&from_toml));
        assert_eq!(from_flags.psi_key, from_toml.psi_key, "psi_key is never serialised, so compare it directly");
        assert_ne!(json(&from_flags), json(&sitevet::Options::default()));
    }

    #[test]
    fn flags_override_the_file() {
        let (mut options, _) = crate::config::parse("max_pages = 7\nbrowser = true").unwrap();
        let cli =
            Cli::try_parse_from(["sitevet", "audit", "https://e.com", "--max-pages", "3", "--browser=false"]).unwrap();
        let Command::Audit(args) = cli.command else { panic!("not audit") };
        args.apply(&mut options);
        assert_eq!((options.max_pages, options.browser), (3, false));
    }
}
