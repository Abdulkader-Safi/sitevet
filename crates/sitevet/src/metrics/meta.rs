//! Social and image meta, from the TS package's `src/metrics/meta.ts`.

use std::sync::Arc;

use scraper::Html;

use super::{EMPTY, cell, num, txt, value};
use crate::report::{MetricCell, ValueStatus};
use crate::rules::kit::{attr, clip};
use crate::{Collector, MetricInput};

const OG_TAGS: [&str; 7] = ["og:title", "og:description", "og:image", "og:url", "og:type", "og:site_name", "og:locale"];
const TWITTER_TAGS: [&str; 6] =
    ["twitter:card", "twitter:title", "twitter:description", "twitter:image", "twitter:site", "twitter:creator"];
const OG_REQUIRED: [&str; 3] = ["og:title", "og:description", "og:image"];
const TWITTER_REQUIRED: [&str; 1] = ["twitter:card"];

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(OpenGraph), Arc::new(Twitter), Arc::new(ImageCoverage)]
}

/// Open Graph uses `property`, Twitter uses `name`, and plenty of sites mix them.
fn meta_content(dom: &Html, key: &str) -> Option<String> {
    attr(dom, &format!(r#"meta[property="{key}"]"#), "content")
        .or_else(|| attr(dom, &format!(r#"meta[name="{key}"]"#), "content"))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Rows of (tag, content, state), how many are present, and the required tags that are missing.
fn tag_rows(dom: &Html, keys: &[&str], required: &[&str]) -> (Vec<Vec<MetricCell>>, usize, Vec<String>) {
    let mut missing = Vec::new();
    let mut present = 0;
    let rows = keys
        .iter()
        .map(|key| {
            let content = meta_content(dom, key);
            let state = match (&content, required.contains(key)) {
                (Some(_), _) => {
                    present += 1;
                    "present"
                }
                (None, true) => {
                    missing.push(key.to_string());
                    "missing"
                }
                (None, false) => "not set",
            };
            vec![cell(*key), cell(content.map_or_else(|| EMPTY.to_string(), |c| clip(&c, 70))), cell(state)]
        })
        .collect();
    (rows, present, missing)
}

fn coverage(present: usize, missing: usize) -> ValueStatus {
    if present == 0 {
        ValueStatus::Missing
    } else if missing == 0 {
        ValueStatus::Ok
    } else {
        ValueStatus::Warn
    }
}

dom_collector!(OpenGraph, "core-seo", "core-seo/open-graph", |cx, dom, html| {
    let (rows, present, missing) = tag_rows(dom, &OG_TAGS, &OG_REQUIRED);
    let note = if missing.is_empty() { String::new() } else { format!("Missing {}", missing.join(", ")) };
    vec![
        value("core-seo/og-coverage", "Open Graph tags", txt(format!("{present} of {}", OG_TAGS.len())))
            .with_status(coverage(present, missing.len()))
            .with_note(note),
        MetricInput::table("core-seo/og-table", "Open Graph", &["Property", "Content", "State"], rows),
    ]
});

dom_collector!(Twitter, "social-media", "social-media/twitter", |cx, dom, html| {
    let (rows, present, missing) = tag_rows(dom, &TWITTER_TAGS, &TWITTER_REQUIRED);
    // Twitter falls back to Open Graph for everything but the card type.
    let note = if present == 0 { "Falls back to Open Graph" } else { "" };
    vec![
        value("social-media/twitter-coverage", "Twitter tags", txt(format!("{present} of {}", TWITTER_TAGS.len())))
            .with_status(coverage(present, missing.len()))
            .with_note(note),
        MetricInput::table("social-media/twitter-table", "Twitter card", &["Name", "Content", "State"], rows),
    ]
});

dom_collector!(ImageCoverage, "images", "images/coverage", |cx, dom, html| {
    let images = &cx.page.images;
    let with_alt = images.iter().filter(|i| i.alt.as_deref().is_some_and(|a| !a.trim().is_empty())).count();
    let decorative = images.iter().filter(|i| i.alt.as_deref() == Some("")).count();
    let without = images.iter().filter(|i| i.alt.is_none()).count();
    let http = images.iter().filter(|i| i.url.as_deref().is_some_and(|u| u.starts_with("http://"))).count();
    let ok_or = |n: usize, bad: ValueStatus| if n == 0 { ValueStatus::Ok } else { bad };
    vec![
        value("images/count", "Images", num(images.len() as f64)),
        value("images/with-alt", "With alt text", num(with_alt as f64))
            .with_status(ok_or(without, ValueStatus::Warn))
            .with_note(format!("{decorative} marked decorative, {without} with no alt attribute")),
        value("images/without-alt", "Missing alt", num(without as f64)).with_status(ok_or(without, ValueStatus::Bad)),
        value("images/http-count", "Loaded over HTTP", num(http as f64))
            .with_status(ok_or(http, ValueStatus::Bad))
            .with_note("Insecure images break the padlock on an HTTPS page"),
    ]
});
