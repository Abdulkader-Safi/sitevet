//! Markup hygiene counters, from the TS package's `src/metrics/markup.ts`.

use std::sync::Arc;

use scraper::Node;

use super::{num, up_to, value};
use crate::Collector;
use crate::report::ValueStatus;
use crate::rules::kit::count;

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(MarkupHygiene), Arc::new(ScriptCount)]
}

fn ok_or_warn(n: usize) -> ValueStatus {
    if n == 0 { ValueStatus::Ok } else { ValueStatus::Warn }
}

dom_collector!(MarkupHygiene, "content", "content/markup-hygiene", |cx, dom, html| {
    let inline = count(dom, "[style]");
    let body_styles = count(dom, "body style");
    // Comment nodes anywhere, the head included.
    let comments = dom.tree.nodes().filter(|n| matches!(n.value(), Node::Comment(_))).count();
    let placeholders = cx
        .page
        .links
        .iter()
        .filter(|l| {
            let h = l.href.trim();
            h.is_empty() || h == "#" || h.to_lowercase().starts_with("javascript:void")
        })
        .count();
    vec![
        value("content/inline-styles", "Inline style attributes", num(inline as f64))
            .with_status(up_to(inline as f64, 20.0, 100.0))
            .with_note("Style belongs in a stylesheet"),
        value("content/body-style-blocks", "Style blocks in body", num(body_styles as f64))
            .with_status(ok_or_warn(body_styles))
            .with_note("CSS after <body> opens can restyle painted content"),
        value("content/html-comments", "HTML comments", num(comments as f64))
            .with_status(up_to(comments as f64, 30.0, 150.0))
            .with_note("Comments ship to every visitor"),
        value("content/placeholder-links", "Placeholder links", num(placeholders as f64))
            .with_status(ok_or_warn(placeholders))
            .with_note(r##"href="#" or empty"##),
    ]
});

dom_collector!(ScriptCount, "performance", "performance/script-count", |cx, dom, html| {
    let scripts = count(dom, "script");
    let external = count(dom, "script[src]");
    vec![
        value("performance/script-tags", "Script tags", num(scripts as f64))
            .with_status(up_to(scripts as f64, 15.0, 40.0))
            .with_note(format!("{external} external, {} inline", scripts - external)),
    ]
});
