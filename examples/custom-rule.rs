//! Adds one custom rule to the built-in set and prints what it found.
//!
//! cargo run -p sitevet --example custom-rule -- https://example.com

use std::sync::Arc;

use sitevet::{Options, Rule, RuleCx, RuleMeta, RuleResult, Severity, Stage, audit};

/// Flags pages that still carry placeholder text.
struct NoLoremIpsum;

impl Rule for NoLoremIpsum {
    fn meta(&self) -> RuleMeta {
        let mut meta =
            RuleMeta::new("content/no-lorem-ipsum", "content", "No placeholder text", Severity::Warning, Stage::Dom);
        meta.fix = Some("Replace the lorem ipsum text with real copy.");
        meta
    }

    fn run(&self, cx: &RuleCx<'_>) -> Vec<RuleResult> {
        if cx.page.text.to_lowercase().contains("lorem ipsum") {
            vec![RuleResult::fail("Page contains lorem ipsum placeholder text")]
        } else {
            vec![RuleResult::pass("No placeholder text")]
        }
    }
}

#[tokio::main]
async fn main() {
    let Some(url) = std::env::args().nth(1) else {
        eprintln!("usage: custom-rule <url>");
        std::process::exit(2);
    };
    let mut rules = sitevet::all_rules();
    rules.push(Arc::new(NoLoremIpsum));
    let mut options = Options::default();
    options.max_pages = 10;
    options.rules = Some(rules);

    match audit(&url, options).await {
        Ok(report) => {
            for page in &report.pages {
                for f in page.findings.iter().filter(|f| f.rule_id == "content/no-lorem-ipsum") {
                    println!("{:?}  {}  {}", f.status, page.url, f.message);
                }
            }
            println!("score {}", report.score);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
}
