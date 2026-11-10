//! Link counts and tables, from the TS package's `src/metrics/links.ts`.

use std::sync::Arc;

use super::{EMPTY, cell, cell_n, num, value};
use crate::report::{MetricCell, ValueStatus};
use crate::rules::kit::{clip, re};
use crate::{Collector, Link, MetricInput};

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(Inventory)]
}

fn scope(l: &Link) -> &'static str {
    if l.internal { "internal" } else { "external" }
}

/// URL, every distinct anchor used for it, how many times it appears, most linked first.
fn by_target(links: &[&Link]) -> Vec<Vec<MetricCell>> {
    let mut targets: Vec<(&str, Vec<&str>, usize, bool)> = Vec::new();
    for l in links {
        let Some(url) = l.url.as_deref() else { continue };
        let i = targets.iter().position(|t| t.0 == url).unwrap_or_else(|| {
            targets.push((url, Vec::new(), 0, l.internal));
            targets.len() - 1
        });
        let t = &mut targets[i];
        t.2 += 1;
        if !l.text.is_empty() && !t.1.contains(&l.text.as_str()) {
            t.1.push(&l.text);
        }
    }
    targets.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(b.0)));
    targets
        .into_iter()
        .map(|(url, anchors, n, internal)| {
            vec![
                cell(clip(url, 70)),
                cell(if anchors.is_empty() { EMPTY.to_string() } else { clip(&anchors.join(" · "), 60) }),
                cell_n(n as f64),
                cell(if internal { "internal" } else { "external" }),
            ]
        })
        .collect()
}

fn link_rows(links: &[&Link]) -> Vec<Vec<MetricCell>> {
    links
        .iter()
        .filter_map(|l| {
            let url = l.url.as_deref()?;
            Some(vec![
                cell(clip(url, 70)),
                cell(if l.text.is_empty() { EMPTY.to_string() } else { clip(&l.text, 50) }),
                cell(scope(l)),
            ])
        })
        .collect()
}

dom_collector!(Inventory, "links", "links/inventory", |cx, dom, html| {
    let links = &cx.page.links;
    let nofollow_re = re(r"\bnofollow\b");
    let resolved: Vec<&Link> = links.iter().filter(|l| l.url.is_some()).collect();
    let internal = resolved.iter().filter(|l| l.internal).count();
    let external = resolved.len() - internal;
    let (nofollow, dofollow): (Vec<&Link>, Vec<&Link>) =
        resolved.iter().copied().partition(|l| nofollow_re.is_match(&l.rel));
    let duplicates: Vec<Vec<MetricCell>> =
        by_target(&resolved).into_iter().filter(|r| matches!(r[2], MetricCell::Number(n) if n > 1.0)).collect();
    let unresolved = links.len() - resolved.len();
    let mut note = format!("{internal} internal, {external} external");
    if unresolved > 0 {
        note.push_str(&format!(", {unresolved} unresolved"));
    }

    let mut out = vec![
        value("links/total", "Links", num(links.len() as f64)).with_note(note),
        value("links/internal-count", "Internal links", num(internal as f64)),
        value("links/external-count", "External links", num(external as f64)),
        value("links/dofollow-count", "Dofollow links", num(dofollow.len() as f64)),
        value("links/nofollow-count", "Nofollow links", num(nofollow.len() as f64)),
        value("links/duplicate-count", "Duplicated targets", num(duplicates.len() as f64))
            .with_status(if duplicates.is_empty() { ValueStatus::Ok } else { ValueStatus::Warn })
            .with_note("URLs linked more than once from this page"),
    ];
    // Tables only when they have rows.
    let cols = ["URL", "Anchor", "Scope"];
    if !dofollow.is_empty() {
        out.push(MetricInput::table("links/dofollow-table", "Dofollow links", &cols, link_rows(&dofollow)));
    }
    if !nofollow.is_empty() {
        out.push(MetricInput::table("links/nofollow-table", "Nofollow links", &cols, link_rows(&nofollow)));
    }
    if !duplicates.is_empty() {
        out.push(MetricInput::table(
            "links/duplicate-table",
            "Duplicated link targets",
            &["URL", "Anchors used", "Count", "Scope"],
            duplicates,
        ));
    }
    out
});
