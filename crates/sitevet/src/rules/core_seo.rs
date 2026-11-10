//! core-seo rules, ported from the TS package's `src/rules/core-seo.ts`.

use std::sync::Arc;

use super::kit::{attr, attr_trim, count, first_text, first_text_collapsed, header, js_len};
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "core-seo";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(TitlePresent),
        Arc::new(TitleLength),
        Arc::new(MetaDescriptionPresent),
        Arc::new(MetaDescriptionLength),
        Arc::new(SingleH1),
        Arc::new(H1Length),
        Arc::new(CanonicalPresent),
        Arc::new(CharsetDeclared),
        Arc::new(ViewportPresent),
        Arc::new(RobotsMetaIndexable),
        Arc::new(OpenGraphBasics),
    ]
}

dom_rule!(
    TitlePresent,
    "core-seo/title-present",
    "Title tag present",
    Error,
    "Add a unique <title> in <head> that summarises the page in 30 to 60 characters.",
    |cx, dom, html| {
        if first_text(dom, "title").is_empty() {
            vec![R::fail("Missing <title>")]
        } else {
            vec![R::pass("Title present")]
        }
    }
);

dom_rule!(
    TitleLength,
    "core-seo/title-length",
    "Title length",
    Warning,
    "Rewrite the title to sit between 30 and 60 characters so it is not truncated in search results.",
    |cx, dom, html| {
        let t = first_text(dom, "title");
        let n = js_len(&t);
        if t.is_empty() {
            return vec![R::warn("No title to measure")];
        }
        if n < 30 {
            return vec![R::warn(format!("Title is short ({n} chars)")).with_evidence(t)];
        }
        if n > 60 {
            return vec![R::warn(format!("Title is long ({n} chars)")).with_evidence(t)];
        }
        vec![R::pass(format!("Title length good ({n} chars)"))]
    }
);

const DESCRIPTION: &str = r#"meta[name="description"]"#;

dom_rule!(
    MetaDescriptionPresent,
    "core-seo/meta-description-present",
    "Meta description present",
    Error,
    r#"Add a <meta name="description"> with a 70 to 160 character summary of the page."#,
    |cx, dom, html| {
        match attr_trim(dom, DESCRIPTION, "content") {
            Some(_) => vec![R::pass("Meta description present")],
            None => vec![R::fail("Missing meta description")],
        }
    }
);

dom_rule!(
    MetaDescriptionLength,
    "core-seo/meta-description-length",
    "Meta description length",
    Warning,
    "Trim or expand the meta description to 70 to 160 characters.",
    |cx, dom, html| {
        let Some(d) = attr_trim(dom, DESCRIPTION, "content") else {
            return vec![R::warn("No description to measure")];
        };
        let n = js_len(&d);
        if n < 70 {
            return vec![R::warn(format!("Description short ({n} chars)"))];
        }
        if n > 160 {
            return vec![R::warn(format!("Description long ({n} chars)"))];
        }
        vec![R::pass(format!("Description length good ({n} chars)"))]
    }
);

dom_rule!(
    SingleH1,
    "core-seo/single-h1",
    "Exactly one H1",
    Warning,
    "Use exactly one <h1> as the page's main heading. Demote the extras to <h2> or below.",
    |cx, dom, html| {
        match count(dom, "h1") {
            1 => vec![R::pass("Exactly one H1")],
            n => vec![R::fail(format!("Found {n} H1 tags"))],
        }
    }
);

dom_rule!(
    H1Length,
    "core-seo/h1-length",
    "H1 length",
    Info,
    "Rewrite the H1 to sit between 10 and 70 characters. One word says nothing; a sentence is not a heading.",
    |cx, dom, html| {
        let h1 = first_text_collapsed(dom, "h1");
        let n = js_len(&h1);
        if h1.is_empty() {
            return vec![R::warn("No H1 to measure")];
        }
        if n < 10 {
            return vec![R::warn(format!("H1 is short ({n} chars)")).with_evidence(h1)];
        }
        if n > 70 {
            return vec![R::warn(format!("H1 is long ({n} chars)")).with_evidence(h1)];
        }
        vec![R::pass(format!("H1 length good ({n} chars)"))]
    }
);

dom_rule!(
    CanonicalPresent,
    "core-seo/canonical-present",
    "Canonical URL present",
    Warning,
    r#"Add <link rel="canonical" href="..."> pointing to the preferred absolute URL of this page."#,
    |cx, dom, html| {
        let Some(c) = attr_trim(dom, r#"link[rel="canonical"]"#, "href") else {
            return vec![R::warn("No canonical link")];
        };
        match url::Url::parse(&cx.page.final_url).and_then(|base| base.join(&c)) {
            Ok(_) => vec![R::pass("Canonical present").with_evidence(c)],
            Err(_) => vec![R::fail("Canonical is not a valid URL").with_evidence(c)],
        }
    }
);

dom_rule!(
    CharsetDeclared,
    "core-seo/charset-declared",
    "Charset declared",
    Warning,
    r#"Add <meta charset="utf-8"> as the first tag in <head>."#,
    |cx, dom, html| {
        let in_header = header(cx.page, "content-type").is_some_and(|ct| ct.to_lowercase().contains("charset="));
        if count(dom, "meta[charset]") > 0 || in_header {
            vec![R::pass("Charset declared")]
        } else {
            vec![R::warn("No charset declared")]
        }
    }
);

dom_rule!(
    ViewportPresent,
    "core-seo/viewport-present",
    "Viewport meta present",
    Warning,
    r#"Add <meta name="viewport" content="width=device-width, initial-scale=1"> for mobile rendering."#,
    |cx, dom, html| {
        match attr(dom, r#"meta[name="viewport"]"#, "content").filter(|v| !v.is_empty()) {
            Some(_) => vec![R::pass("Viewport present")],
            None => vec![R::fail("Missing viewport meta")],
        }
    }
);

dom_rule!(
    RobotsMetaIndexable,
    "core-seo/robots-meta-indexable",
    "Page is indexable",
    Error,
    "Remove noindex from the robots meta tag if this page should appear in search.",
    |cx, dom, html| {
        let r = attr(dom, r#"meta[name="robots"]"#, "content").unwrap_or_default().to_lowercase();
        if r.contains("noindex") {
            vec![R::fail("Robots meta has noindex").with_evidence(r)]
        } else {
            vec![R::pass("No noindex in robots meta")]
        }
    }
);

dom_rule!(
    OpenGraphBasics,
    "core-seo/open-graph-basics",
    "Open Graph basics",
    Info,
    "Add og:title, og:description, and og:image meta tags so the page previews well when shared.",
    |cx, dom, html| {
        let missing: Vec<&str> = ["og:title", "og:description", "og:image"]
            .into_iter()
            .filter(|p| attr(dom, &format!(r#"meta[property="{p}"]"#), "content").is_none_or(|c| c.is_empty()))
            .collect();
        if missing.is_empty() {
            vec![R::pass("og:title, og:description, og:image present")]
        } else {
            vec![R::warn(format!("Missing Open Graph tags: {}", missing.join(", ")))]
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Fail, Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn title() {
        let good = page("<title>A clear, well sized page title for testing</title>");
        assert_eq!(good.status("core-seo/title-present"), Pass);
        assert_eq!(good.status("core-seo/title-length"), Pass);
        let none = page("<p>no title</p>");
        assert_eq!(none.status("core-seo/title-present"), Fail);
        assert_eq!(none.message("core-seo/title-length"), "No title to measure");
        let short = page("<title>Hi</title>");
        assert_eq!(short.message("core-seo/title-length"), "Title is short (2 chars)");
        assert_eq!(page(&format!("<title>{}</title>", "x".repeat(61))).status("core-seo/title-length"), Warn);
    }

    #[test]
    fn meta_description() {
        let d = |s: &str| page(&format!(r#"<meta name="description" content="{s}">"#));
        assert_eq!(d(&"x".repeat(80)).status("core-seo/meta-description-present"), Pass);
        assert_eq!(d(&"x".repeat(80)).status("core-seo/meta-description-length"), Pass);
        assert_eq!(d("  ").status("core-seo/meta-description-present"), Fail);
        assert_eq!(d("short").message("core-seo/meta-description-length"), "Description short (5 chars)");
        assert_eq!(d(&"x".repeat(161)).status("core-seo/meta-description-length"), Warn);
    }

    #[test]
    fn h1() {
        assert_eq!(page("<h1>A heading of fine length</h1>").status("core-seo/single-h1"), Pass);
        assert_eq!(page("<h1>A</h1><h1>B</h1>").message("core-seo/single-h1"), "Found 2 H1 tags");
        assert_eq!(page("<p>x</p>").status("core-seo/single-h1"), Fail);
        assert_eq!(page("<h1>A   heading\n of fine length</h1>").status("core-seo/h1-length"), Pass);
        assert_eq!(page("<h1>Short</h1>").message("core-seo/h1-length"), "H1 is short (5 chars)");
        assert_eq!(page("<p>x</p>").status("core-seo/h1-length"), Warn);
    }

    #[test]
    fn canonical() {
        let c = page(r#"<link rel="canonical" href="https://example.com/good">"#);
        assert_eq!(c.status("core-seo/canonical-present"), Pass);
        assert_eq!(page("<p>x</p>").status("core-seo/canonical-present"), Warn);
        assert_eq!(page(r#"<link rel="canonical" href="http://[bad">"#).status("core-seo/canonical-present"), Fail);
    }

    #[test]
    fn viewport() {
        let good = page(r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#);
        assert_eq!(good.status("core-seo/viewport-present"), Pass);
        let bad = page("<p>No viewport here</p>");
        assert_eq!(bad.status("core-seo/viewport-present"), Fail);
        assert_eq!(bad.message("core-seo/viewport-present"), "Missing viewport meta");
    }

    #[test]
    fn charset_robots_og() {
        assert_eq!(page(r#"<meta charset="utf-8">"#).status("core-seo/charset-declared"), Pass);
        assert_eq!(
            page("<p>x</p>").header("content-type", "text/html; charset=UTF-8").status("core-seo/charset-declared"),
            Pass
        );
        assert_eq!(page("<p>x</p>").status("core-seo/charset-declared"), Warn);
        assert_eq!(page(r#"<meta name="robots" content="NOINDEX">"#).status("core-seo/robots-meta-indexable"), Fail);
        assert_eq!(page(r#"<meta name="robots" content="index">"#).status("core-seo/robots-meta-indexable"), Pass);
        let og = r#"<meta property="og:title" content="a"><meta property="og:description" content="b"><meta property="og:image" content="c">"#;
        assert_eq!(page(og).status("core-seo/open-graph-basics"), Pass);
        assert_eq!(
            page(r#"<meta property="og:title" content="a">"#).message("core-seo/open-graph-basics"),
            "Missing Open Graph tags: og:description, og:image"
        );
    }
}
