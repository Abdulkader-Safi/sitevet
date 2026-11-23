//! The tech stack per page (FR-TECH-01, FR-TECH-02): every vendor the signatures found, which
//! replaces `analytics/tag-present`'s single yes or no in the measurements. The rule stays.

use std::sync::Arc;

use super::cell;
use crate::{Collector, MetricInput};

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(TechStack)]
}

/// The label a category gets in tables.
pub(crate) fn kind(category: &str) -> &str {
    match category {
        "analytics" => "Analytics",
        "ads" => "Ad pixel",
        "cms" => "CMS",
        "framework" => "Framework or library",
        "cdn" => "CDN or host",
        other => other,
    }
}

dom_collector!(TechStack, "analytics", "analytics/tech-stack", |cx, dom, html| {
    vec![
        MetricInput::table(
            "analytics/tech-stack",
            "Tech stack",
            &["Vendor", "Type", "Found by"],
            cx.page.tech.iter().map(|t| vec![cell(&t.name), cell(kind(&t.category)), cell(&t.evidence)]).collect(),
        )
        .with_note(
            "Analytics, ad pixels, CMS, frameworks and CDN, from script URLs, markup, headers, cookies and meta tags",
        ),
    ]
});

#[cfg(test)]
mod tests {
    use crate::rules::testkit::{fixture, page, rows};

    fn vendors(html: &str) -> Vec<String> {
        let p = page(html);
        rows(&p.metric("analytics/tech-stack"))
            .into_iter()
            .map(|r| match &r[0] {
                crate::MetricCell::Text(t) => t.clone(),
                other => panic!("{other:?}"),
            })
            .collect()
    }

    /// FR-TECH-01: GA4 and the Meta pixel, and nothing else.
    #[test]
    fn ga4_and_meta_pixel_exactly() {
        assert_eq!(vendors(&fixture("tracking.html")), ["Google Analytics", "Meta Pixel"]);
    }

    /// No signature fires on the other fixture pages: none of them use these vendors.
    #[test]
    fn no_false_positives_across_the_fixture_pages() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages");
        for entry in std::fs::read_dir(dir).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            if name == "tracking.html" {
                continue;
            }
            assert_eq!(vendors(&fixture(&name)), Vec::<String>::new(), "{name}");
        }
    }

    #[test]
    fn each_kind_of_signal() {
        let ads = r#"<script async src="https://www.googletagmanager.com/gtag/js?id=AW-1234567"></script>"#;
        assert_eq!(vendors(ads), ["Google Ads"]);
        assert_eq!(vendors(r#"<script src="https://code.jquery.com/jquery-3.7.1.min.js"></script>"#), ["jQuery"]);
        assert_eq!(
            vendors(r#"<div id="__next"></div><script id="__NEXT_DATA__" type="application/json">{}</script>"#),
            ["React", "Next.js"]
        );
        assert_eq!(vendors(r#"<meta name="generator" content="Ghost 5.80">"#), ["Ghost"]);
        let mut laravel = page("<p>x</p>");
        laravel.headers.push(("set-cookie".into(), "laravel_session=abc; path=/; httponly".into()));
        let found = rows(&laravel.metric("analytics/tech-stack"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0][2], crate::MetricCell::Text("cookie laravel_session".into()));
        // A renamed session cookie with Laravel's encrypted value. The value never reaches the report.
        let mut renamed = page("<p>x</p>");
        renamed.headers.push(("set-cookie".into(), "my-app-session=eyJpdiI6IkdC; path=/; secure".into()));
        let found = rows(&renamed.metric("analytics/tech-stack"));
        assert_eq!(found[0][2], crate::MetricCell::Text("cookie my-app-session".into()));
        // Another framework's session cookie is not Laravel.
        let mut other = page("<p>x</p>");
        other.headers.push(("set-cookie".into(), "shop_session=abc123; path=/".into()));
        assert!(rows(&other.metric("analytics/tech-stack")).is_empty());
    }
}
