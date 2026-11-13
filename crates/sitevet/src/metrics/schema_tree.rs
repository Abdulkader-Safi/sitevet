//! The structured data tree (FR-AEO-06): every item from JSON-LD, microdata and RDFa, nested,
//! with each node marked complete or listing the key properties it lacks.

use std::sync::Arc;

use super::{EMPTY, cell, cell_n, num, value};
use crate::report::ValueStatus;
use crate::{Collector, MetricInput, SchemaNode, SchemaSource, SchemaValue};

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(SchemaTree)]
}

// ponytail: a short list of the properties Google's rich results lean on per type. Types not
// listed are marked present without a check. Grow the list when a rule needs one.
const KEY_PROPERTIES: [(&str, &[&str]); 17] = [
    ("Article", &["headline", "author", "datePublished", "image"]),
    ("NewsArticle", &["headline", "author", "datePublished", "image"]),
    ("BlogPosting", &["headline", "author", "datePublished", "image"]),
    ("Organization", &["name", "url", "logo"]),
    ("LocalBusiness", &["name", "address", "telephone"]),
    ("Person", &["name"]),
    ("WebSite", &["name", "url"]),
    ("Product", &["name", "offers"]),
    ("Offer", &["price", "priceCurrency"]),
    ("FAQPage", &["mainEntity"]),
    ("Question", &["name", "acceptedAnswer"]),
    ("Answer", &["text"]),
    ("HowTo", &["name", "step"]),
    ("HowToStep", &["text"]),
    ("BreadcrumbList", &["itemListElement"]),
    ("PostalAddress", &["streetAddress", "addressLocality"]),
    ("GeoCoordinates", &["latitude", "longitude"]),
];

/// The key properties this node lacks.
pub(crate) fn missing(node: &SchemaNode) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for (ty, props) in KEY_PROPERTIES {
        if node.is(ty) {
            for &p in props {
                if !node.has(p) && !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn rows(node: &SchemaNode, depth: usize, source: Option<SchemaSource>, out: &mut Vec<Vec<crate::MetricCell>>) {
    let name = if node.types.is_empty() { "(untyped)".to_string() } else { node.types.join(", ") };
    let gaps = missing(node);
    out.push(vec![
        cell(format!("{}{name}", "- ".repeat(depth))),
        cell(match source {
            Some(SchemaSource::JsonLd) => "JSON-LD",
            Some(SchemaSource::Microdata) => "microdata",
            Some(SchemaSource::Rdfa) => "RDFa",
            None => EMPTY,
        }),
        cell_n(node.properties.len() as f64),
        cell(if gaps.is_empty() { "present" } else { "incomplete" }),
        cell(if gaps.is_empty() { EMPTY.to_string() } else { gaps.join(", ") }),
    ]);
    for p in &node.properties {
        if let SchemaValue::Node(child) = &p.value {
            rows(child, depth + 1, None, out);
        }
    }
}

dom_collector!(SchemaTree, "structured-data", "structured-data/schema-tree", |cx, dom, html| {
    let items = &cx.page.schema;
    let count = |s: SchemaSource| items.iter().filter(|i| i.source == s).count();
    let mut out = vec![
        value("structured-data/schema-items", "Structured data items", num(items.len() as f64))
            .with_status(if items.is_empty() { ValueStatus::Missing } else { ValueStatus::Ok })
            .with_note(format!(
                "JSON-LD {}, microdata {}, RDFa {}",
                count(SchemaSource::JsonLd),
                count(SchemaSource::Microdata),
                count(SchemaSource::Rdfa)
            )),
    ];
    if !items.is_empty() {
        let mut table = Vec::new();
        for i in items {
            rows(&i.node, 0, Some(i.source), &mut table);
        }
        out.push(
            MetricInput::table_capped(
                "structured-data/schema-tree",
                "Structured data tree",
                &["Type", "Source", "Properties", "Status", "Missing"],
                table,
                60,
            )
            .with_note("Nested items are indented under their parent"),
        );
    }
    out
});

#[cfg(test)]
mod tests {
    use crate::rules::testkit::{n, page, row, rows, value};

    const FAQ_JSONLD: &str = r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"FAQPage",
        "mainEntity":[{"@type":"Question","name":"Why compost?","acceptedAnswer":{"@type":"Answer","text":"It feeds soil."}},
        {"@type":"Question","name":"When?"}]}</script>"#;
    const FAQ_MICRODATA: &str = r#"<div itemscope itemtype="https://schema.org/FAQPage">
        <div itemprop="mainEntity" itemscope itemtype="https://schema.org/Question"><h3 itemprop="name">Why compost?</h3>
          <div itemprop="acceptedAnswer" itemscope itemtype="https://schema.org/Answer"><p itemprop="text">It feeds soil.</p></div></div>
        <div itemprop="mainEntity" itemscope itemtype="https://schema.org/Question"><h3 itemprop="name">When?</h3></div></div>"#;

    #[test]
    fn microdata_twin_gives_the_same_tree_as_json_ld() {
        let strip_source = |html: &str| {
            let mut all = rows(&page(html).metric("structured-data/schema-tree"));
            all.iter_mut().for_each(|r| {
                r.remove(1);
            });
            all
        };
        let tree = strip_source(FAQ_JSONLD);
        assert_eq!(tree, strip_source(FAQ_MICRODATA));
        assert_eq!(
            tree,
            [
                row(&["FAQPage", "2", "present", "-"]),
                row(&["- Question", "2", "present", "-"]),
                row(&["- - Answer", "1", "present", "-"]),
                row(&["- Question", "1", "incomplete", "acceptedAnswer"]),
            ]
        );
        let json = page(FAQ_JSONLD);
        assert_eq!(value(&json.metric("structured-data/schema-items")).0, n(1.0));
        assert_eq!(rows(&json.metric("structured-data/schema-tree"))[0][1], row(&["JSON-LD"])[0]);
    }
}
