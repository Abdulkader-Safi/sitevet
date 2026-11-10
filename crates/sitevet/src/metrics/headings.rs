//! The heading outline, from the TS package's `src/metrics/headings.ts`.

use std::sync::Arc;

use scraper::ElementRef;

use super::{EMPTY, cell, cell_n, num, value};
use crate::report::ValueStatus;
use crate::rules::kit::{all, clip, collapse, count, sel, text};
use crate::{Collector, MetricInput};

const LEVELS: [&str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
const HEADINGS: &str = "h1,h2,h3,h4,h5,h6";
/// Elements worth naming under a heading.
const NOTABLE: [&str; 9] = ["img", "ul", "ol", "table", "video", "iframe", "form", "pre", "blockquote"];

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(Headings)]
}

/// The siblings after `h` up to the next heading: the section it opens.
fn section(h: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    let heading = sel(HEADINGS);
    h.next_siblings().filter_map(ElementRef::wrap).take_while(|e| !heading.matches(e)).collect()
}

dom_collector!(Headings, "content", "content/headings", |cx, dom, html| {
    let counts: Vec<usize> = LEVELS.iter().map(|l| count(dom, l)).collect();
    let total: usize = counts.iter().sum();
    let outline: Vec<_> = all(dom, HEADINGS)
        .into_iter()
        .map(|h| {
            let tag = h.value().name().to_uppercase();
            let level = usize::from(tag.as_bytes()[1] - b'0');
            let words = collapse(&text(h));
            let notable: Vec<String> = NOTABLE
                .iter()
                .filter_map(|n| {
                    let s = sel(n);
                    let found: usize =
                        section(h).iter().map(|e| e.select(&s).count() + usize::from(s.matches(e))).sum();
                    (found > 0).then(|| format!("{}: {found}", n.to_uppercase()))
                })
                .collect();
            vec![
                cell(tag),
                // Indented by level so the outline reads as a tree in a flat table.
                cell(format!(
                    "{}{}",
                    "- ".repeat(level - 1),
                    if words.is_empty() { "(empty)".into() } else { clip(&words, 70) }
                )),
                cell(if notable.is_empty() { EMPTY.to_string() } else { notable.join(", ") }),
            ]
        })
        .collect();
    let note =
        LEVELS.iter().zip(&counts).map(|(l, n)| format!("{} {n}", l.to_uppercase())).collect::<Vec<_>>().join(" · ");
    let mut out = vec![
        value("content/heading-count", "Headings", num(total as f64))
            .with_status(if total == 0 { ValueStatus::Missing } else { ValueStatus::Ok })
            .with_note(note),
        MetricInput::table(
            "content/heading-levels",
            "Headings per level",
            &["Level", "Count"],
            LEVELS.iter().zip(&counts).map(|(l, n)| vec![cell(l.to_uppercase()), cell_n(*n as f64)]).collect(),
        ),
    ];
    if !outline.is_empty() {
        out.push(MetricInput::table_capped(
            "content/heading-outline",
            "Heading outline",
            &["Level", "Heading", "Contains"],
            outline,
            40,
        ));
    }
    out
});
