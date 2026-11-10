//! `sitevet init`: a commented `sitevet.toml` with every setting at its default.

use std::path::Path;
use std::process::ExitCode;

use anyhow::bail;

use crate::args::InitArgs;

pub const TEMPLATE: &str = r#"# sitevet settings. Every key is shown at its default; change what you need.
# Command-line flags override this file. Flag names are these keys with dashes:
# max_pages = 50 is the same as --max-pages 50.

# Categories to run (empty means all) and categories to skip, e.g. ["security", "links"].
only = []
skip = []

# Crawl limits. max_pages is capped at 10000.
max_pages = 100
max_depth = 5
# Requests in flight per host, and the gap between requests to one host.
concurrency = 5
delay_ms = 0
timeout_ms = 15000
# user_agent = "sitevet/<version> (+https://github.com/Abdulkader-Safi/sitevet)"

# robots.txt is obeyed unless ignore_robots is true. The sitemap seeds the crawl.
ignore_robots = false
crawl_sitemap = true
# Private and loopback addresses are allowed only when the start URL is private.
# allow_private_network = false

# Rendered checks in a local Chrome: accessibility (axe-core) and Core Web Vitals.
browser = false
# chrome_path = "/path/to/chrome"
render_crawl = false
screenshots = false
# screenshot_dir = "screenshots"

# PageSpeed Insights, used when browser is off. Prefer the SITEVET_PSI_KEY environment
# variable over writing the key here.
# psi_key = ""
psi_max_pages = 5

# Image size probes.
media = false
media_max_requests = 200

# Thresholds for site-wide and performance checks.
click_depth_warn = 3
slow_resource_ms = 300
runtime_threads = 2

[cli]
# Report format: "html", "md" or "json". Default: from the --out extension, else html.
# format = "html"
# out = "report.html"
# Exit 1 when the score is below fail_under, or when a finding fails at fail_on ("error" or "warning").
# fail_under = 80
# fail_on = "error"
# Compare each run with a saved run ("last" for this site, or a run ID) or a JSON report file.
# baseline = "last"
# Save each run to history (sitevet history lists them).
# save = true
"#;

pub fn run(args: &InitArgs) -> anyhow::Result<ExitCode> {
    let path = Path::new("sitevet.toml");
    if path.exists() && !args.force {
        bail!("sitevet.toml already exists; use --force to overwrite it");
    }
    std::fs::write(path, TEMPLATE)?;
    anstream::eprintln!("wrote sitevet.toml");
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    /// FR-CLI-07: the written file parses back into default Options.
    #[test]
    fn template_is_the_defaults() {
        let (options, cli) = crate::config::parse(super::TEMPLATE).unwrap();
        let json = |o: &sitevet::Options| serde_json::to_value(o).unwrap();
        assert_eq!(json(&options), json(&sitevet::Options::default()));
        assert!(cli.format.is_none() && cli.out.is_none());
        // Every key is mentioned, set or commented.
        for key in sitevet::Options::KEYS {
            assert!(super::TEMPLATE.contains(&format!("{key} = ")), "{key} missing from the template");
        }
    }
}
