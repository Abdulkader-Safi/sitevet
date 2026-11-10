//! structured-data rules, ported from the TS package's `src/rules/structured-data.ts`.

use std::sync::Arc;

use scraper::Html;
use serde_json::Value;

use super::kit::{all, re, text};
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "structured-data";
pub(crate) const JSON_LD: &str = r#"script[type="application/ld+json"]"#;
const ORG_TYPES: &str = r#""@type"\s*:\s*"(Organization|LocalBusiness)""#;
const ARTICLE_TYPES: &str = r#""@type"\s*:\s*"(Article|NewsArticle|BlogPosting)""#;

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(OrgOrWebsite),
        Arc::new(ContextDeclared),
        Arc::new(OrgSameas),
        Arc::new(OrgLogo),
        Arc::new(ArticleRequired),
        Arc::new(ArticleImage),
        Arc::new(JsonldPresent),
        Arc::new(JsonldValid),
        Arc::new(JsonldType),
        Arc::new(BreadcrumbValid),
        Arc::new(DuplicateJsonld),
    ]
}

/// Each JSON-LD block: its parsed value, or None when it does not parse.
pub(crate) fn blocks(dom: &Html) -> Vec<Option<Value>> {
    all(dom, JSON_LD).into_iter().map(|el| serde_json::from_str(&text(el)).ok()).collect()
}

fn mentions(v: &Value, key: &str) -> bool {
    serde_json::to_string(v).is_ok_and(|s| s.contains(&format!("\"{key}\"")))
}

dom_rule!(
    OrgOrWebsite,
    "structured-data/org-or-website",
    "Organization or WebSite schema",
    Warning,
    "Add Organization or WebSite JSON-LD so search engines understand the site entity and can show a knowledge panel.",
    |cx, dom, html| {
        if re(r#""@type"\s*:\s*"(Organization|WebSite|LocalBusiness)""#).is_match(html) {
            vec![R::pass("Organization or WebSite schema present")]
        } else {
            vec![R::warn("No Organization or WebSite schema")]
        }
    }
);

dom_rule!(
    ContextDeclared,
    "structured-data/context-declared",
    "JSON-LD @context",
    Info,
    r#"Each JSON-LD block should declare "@context": "https://schema.org"."#,
    |cx, dom, html| {
        let blocks = blocks(dom);
        if blocks.is_empty() {
            return vec![R::pass("No JSON-LD")];
        }
        // Blocks that do not parse are jsonld-valid's business.
        let missing = blocks.iter().flatten().filter(|v| !mentions(v, "@context")).count();
        if missing == 0 {
            vec![R::pass("All JSON-LD blocks declare @context")]
        } else {
            vec![R::warn(format!("{missing} JSON-LD block(s) missing @context"))]
        }
    }
);

dom_rule!(
    OrgSameas,
    "structured-data/org-sameas",
    "Organization sameAs links",
    Info,
    "Add a sameAs array to your Organization schema linking to your official social and profile URLs. It strengthens entity recognition.",
    |cx, dom, html| {
        if !re(ORG_TYPES).is_match(html) {
            return vec![R::pass("No Organization schema")];
        }
        if re(r#""sameAs"\s*:"#).is_match(html) {
            vec![R::pass("Organization declares sameAs")]
        } else {
            vec![R::warn("Organization schema has no sameAs links")]
        }
    }
);

dom_rule!(
    OrgLogo,
    "structured-data/org-logo",
    "Organization logo",
    Info,
    "Add a logo property to your Organization schema so Google can use it in search features.",
    |cx, dom, html| {
        if !re(ORG_TYPES).is_match(html) {
            return vec![R::pass("No Organization schema")];
        }
        if re(r#""logo"\s*:"#).is_match(html) {
            vec![R::pass("Organization declares a logo")]
        } else {
            vec![R::warn("Organization schema has no logo")]
        }
    }
);

dom_rule!(
    ArticleRequired,
    "structured-data/article-required",
    "Article required fields",
    Warning,
    "Article schema needs headline, image, datePublished, and author for rich results. Add the missing fields.",
    |cx, dom, html| {
        if !re(ARTICLE_TYPES).is_match(html) {
            return vec![R::pass("No article schema")];
        }
        let missing: Vec<&str> = ["headline", "image", "datePublished", "author"]
            .into_iter()
            .filter(|k| !re(&format!(r#""{k}"\s*:"#)).is_match(html))
            .collect();
        if missing.is_empty() {
            vec![R::pass("Article schema has the required fields")]
        } else {
            vec![R::warn(format!("Article schema missing: {}", missing.join(", ")))]
        }
    }
);

dom_rule!(
    ArticleImage,
    "structured-data/article-image",
    "Article schema has image",
    Info,
    "Article and NewsArticle schema should include an image property for rich results.",
    |cx, dom, html| {
        if !re(ARTICLE_TYPES).is_match(html) {
            return vec![R::pass("No article schema")];
        }
        if re(r#""image"\s*:"#).is_match(html) {
            vec![R::pass("Article schema includes image")]
        } else {
            vec![R::warn("Article schema missing image")]
        }
    }
);

dom_rule!(
    JsonldPresent,
    "structured-data/jsonld-present",
    "JSON-LD present",
    Info,
    "Add JSON-LD structured data describing the page, e.g. Article, Product, or Organization.",
    |cx, dom, html| {
        match all(dom, JSON_LD).len() {
            0 => vec![R::warn("No JSON-LD structured data")],
            n => vec![R::pass(format!("{n} JSON-LD block(s)"))],
        }
    }
);

dom_rule!(
    JsonldValid,
    "structured-data/jsonld-valid",
    "JSON-LD parses",
    Warning,
    "Fix the JSON syntax in the flagged ld+json blocks so they parse cleanly.",
    |cx, dom, html| {
        let blocks = blocks(dom);
        if blocks.is_empty() {
            return vec![R::pass("No JSON-LD to validate")];
        }
        match blocks.iter().filter(|b| b.is_none()).count() {
            0 => vec![R::pass("All JSON-LD blocks parse")],
            bad => vec![R::fail(format!("{bad} JSON-LD block(s) invalid"))],
        }
    }
);

dom_rule!(
    JsonldType,
    "structured-data/jsonld-type",
    "JSON-LD has @type",
    Info,
    "Include an @type in each JSON-LD block so search engines know what it describes.",
    |cx, dom, html| {
        let blocks = blocks(dom);
        if blocks.is_empty() {
            return vec![R::pass("No JSON-LD")];
        }
        if blocks.iter().flatten().any(|v| mentions(v, "@type")) {
            vec![R::pass("JSON-LD declares @type")]
        } else {
            vec![R::warn("No @type in JSON-LD")]
        }
    }
);

dom_rule!(
    BreadcrumbValid,
    "structured-data/breadcrumb-valid",
    "Breadcrumb schema",
    Info,
    "Add BreadcrumbList structured data to show the page's position in the site.",
    |cx, dom, html| {
        if html.contains("BreadcrumbList") {
            vec![R::pass("BreadcrumbList present")]
        } else {
            vec![R::info("No breadcrumb schema")]
        }
    }
);

/// JavaScript truthiness, for `.filter(Boolean)`.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

dom_rule!(
    DuplicateJsonld,
    "structured-data/duplicate-jsonld",
    "Duplicate JSON-LD types",
    Info,
    "Merge or remove duplicate schema blocks of the same @type.",
    |cx, dom, html| {
        let mut types: Vec<Value> = Vec::new();
        for v in blocks(dom).into_iter().flatten() {
            let found: Vec<Value> = match &v {
                Value::Array(items) => items.iter().map(|x| x.get("@type").cloned().unwrap_or(Value::Null)).collect(),
                other => vec![other.get("@type").cloned().unwrap_or(Value::Null)],
            };
            types.extend(found.into_iter().filter(truthy));
        }
        // JavaScript compares arrays and objects by reference, so only scalars can repeat.
        let mut dup: Vec<String> = Vec::new();
        for (i, t) in types.iter().enumerate() {
            let scalar = !matches!(t, Value::Array(_) | Value::Object(_));
            if scalar && types[..i].contains(t) {
                let label = t.as_str().map_or_else(|| t.to_string(), str::to_string);
                if !dup.contains(&label) {
                    dup.push(label);
                }
            }
        }
        if dup.is_empty() {
            vec![R::pass("No duplicate schema types")]
        } else {
            vec![R::info(format!("Duplicate schema type(s): {}", dup.join(", ")))]
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Fail, Info, Pass, Warn};
    use crate::rules::testkit::page;

    fn ld(json: &str) -> String {
        format!(r#"<script type="application/ld+json">{json}</script>"#)
    }

    #[test]
    fn organization() {
        let org = page(&ld(
            r#"{"@context":"https://schema.org","@type":"Organization","logo":"/l.png","sameAs":["https://x.com/a"]}"#,
        ));
        for id in ["org-or-website", "org-sameas", "org-logo"] {
            assert_eq!(org.status(&format!("structured-data/{id}")), Pass, "{id}");
        }
        let bare = page(&ld(r#"{"@type":"Organization"}"#));
        assert_eq!(bare.status("structured-data/org-sameas"), Warn);
        assert_eq!(bare.status("structured-data/org-logo"), Warn);
        assert_eq!(page("<p>x</p>").status("structured-data/org-or-website"), Warn);
    }

    #[test]
    fn article() {
        let full = page(&ld(r#"{"@type":"Article","headline":"h","image":"i","datePublished":"d","author":"a"}"#));
        assert_eq!(full.status("structured-data/article-required"), Pass);
        assert_eq!(full.status("structured-data/article-image"), Pass);
        let thin = page(&ld(r#"{"@type":"BlogPosting","headline":"h"}"#));
        assert_eq!(
            thin.message("structured-data/article-required"),
            "Article schema missing: image, datePublished, author"
        );
        assert_eq!(thin.status("structured-data/article-image"), Warn);
    }

    #[test]
    fn blocks_parse_type_context() {
        let good = page(&ld(r#"{"@context":"https://schema.org","@type":"Thing"}"#));
        for id in ["jsonld-present", "jsonld-valid", "jsonld-type", "context-declared"] {
            assert_eq!(good.status(&format!("structured-data/{id}")), Pass, "{id}");
        }
        let none = page("<p>x</p>");
        assert_eq!(none.status("structured-data/jsonld-present"), Warn);
        let broken = page(&(ld("{ not valid json }") + &ld(r#"{"name":"x"}"#)));
        assert_eq!(broken.message("structured-data/jsonld-valid"), "1 JSON-LD block(s) invalid");
        assert_eq!(broken.status("structured-data/jsonld-valid"), Fail);
        assert_eq!(broken.status("structured-data/jsonld-type"), Warn);
        assert_eq!(broken.message("structured-data/context-declared"), "1 JSON-LD block(s) missing @context");
    }

    #[test]
    fn breadcrumb_and_duplicates() {
        assert_eq!(page(&ld(r#"{"@type":"BreadcrumbList"}"#)).status("structured-data/breadcrumb-valid"), Pass);
        assert_eq!(page("<p>x</p>").status("structured-data/breadcrumb-valid"), Info);
        let dup = page(&(ld(r#"{"@type":"Article"}"#) + &ld(r#"[{"@type":"Article"},{"@type":"Person"}]"#)));
        assert_eq!(dup.message("structured-data/duplicate-jsonld"), "Duplicate schema type(s): Article");
        assert_eq!(page(&ld(r#"{"@type":"Article"}"#)).status("structured-data/duplicate-jsonld"), Pass);
    }
}
