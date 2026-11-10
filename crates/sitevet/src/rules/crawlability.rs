//! crawlability rules, ported from the TS package's `src/rules/crawlability.ts`.

use std::sync::Arc;

use super::kit::{attr, attr_trim, header, re};
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "crawlability";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(RobotsExists),
        Arc::new(RobotsValid),
        Arc::new(SitemapPresent),
        Arc::new(PageIndexable),
        Arc::new(CanonicalValid),
        Arc::new(XRobotsTag),
    ]
}

dom_rule!(
    RobotsExists,
    "crawlability/robots-exists",
    "robots.txt exists",
    Warning,
    "Add a robots.txt at the site root to guide crawlers.",
    |cx, dom, html| {
        if cx.site.robots.exists { vec![R::pass("robots.txt found")] } else { vec![R::warn("No robots.txt")] }
    }
);

dom_rule!(
    RobotsValid,
    "crawlability/robots-valid",
    "robots.txt valid",
    Info,
    "Add valid directives (User-agent, Disallow, Allow, Sitemap) to robots.txt.",
    |cx, dom, html| {
        if !cx.site.robots.exists {
            return vec![R::pass("No robots.txt to validate")];
        }
        if re(r"(?i)(user-agent:|disallow:|allow:|sitemap:)").is_match(&cx.site.robots.content) {
            vec![R::pass("robots.txt has valid directives")]
        } else {
            vec![R::warn("robots.txt has no recognizable directives")]
        }
    }
);

dom_rule!(
    SitemapPresent,
    "crawlability/sitemap-present",
    "Sitemap present",
    Warning,
    "Publish an XML sitemap and reference it from robots.txt.",
    |cx, dom, html| {
        if cx.site.sitemap.exists {
            vec![R::pass(format!("Sitemap with {} URLs", cx.site.sitemap.urls.len()))]
        } else {
            vec![R::warn("No sitemap found")]
        }
    }
);

dom_rule!(
    PageIndexable,
    "crawlability/page-indexable",
    "Page indexable",
    Error,
    "Remove noindex from the page meta or its X-Robots-Tag header if it should be indexed.",
    |cx, dom, html| {
        let meta = attr(dom, r#"meta[name="robots"]"#, "content").unwrap_or_default().to_lowercase();
        let x = header(cx.page, "x-robots-tag").unwrap_or_default().to_lowercase();
        if meta.contains("noindex") || x.contains("noindex") {
            vec![R::fail("Page is set to noindex")]
        } else {
            vec![R::pass("Page is indexable")]
        }
    }
);

dom_rule!(
    CanonicalValid,
    "crawlability/canonical-valid",
    "Canonical valid",
    Warning,
    "Use an absolute, valid URL in the canonical link.",
    |cx, dom, html| {
        let Some(c) = attr_trim(dom, r#"link[rel="canonical"]"#, "href") else {
            return vec![R::warn("No canonical to validate")];
        };
        match url::Url::parse(&cx.page.final_url).and_then(|base| base.join(&c)) {
            Ok(u) => vec![R::pass("Canonical is absolute and valid").with_evidence(u)],
            Err(_) => vec![R::fail("Canonical is invalid").with_evidence(c)],
        }
    }
);

dom_rule!(
    XRobotsTag,
    "crawlability/x-robots-tag",
    "X-Robots-Tag header",
    Info,
    "Remove noindex or nofollow from the X-Robots-Tag header if the page should rank.",
    |cx, dom, html| {
        let x = header(cx.page, "x-robots-tag").unwrap_or_default().to_lowercase();
        if x.is_empty() {
            return vec![R::pass("No restrictive X-Robots-Tag")];
        }
        if x.contains("noindex") || x.contains("nofollow") {
            vec![R::warn(format!("X-Robots-Tag: {x}"))]
        } else {
            vec![R::pass(format!("X-Robots-Tag: {x}"))]
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Fail, Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn robots_and_sitemap() {
        let p = page("<p>x</p>");
        assert_eq!(p.status("crawlability/robots-exists"), Pass);
        assert_eq!(p.status("crawlability/robots-valid"), Pass);
        assert_eq!(p.message("crawlability/sitemap-present"), "Sitemap with 1 URLs");
        let mut none = page("<p>x</p>");
        none.site.robots.exists = false;
        none.site.sitemap.exists = false;
        assert_eq!(none.status("crawlability/robots-exists"), Warn);
        assert_eq!(none.status("crawlability/robots-valid"), Pass);
        assert_eq!(none.status("crawlability/sitemap-present"), Warn);
        let mut junk = page("<p>x</p>");
        junk.site.robots.content = "hello".into();
        assert_eq!(junk.status("crawlability/robots-valid"), Warn);
    }

    #[test]
    fn indexable_canonical_header() {
        assert_eq!(page("<p>x</p>").status("crawlability/page-indexable"), Pass);
        assert_eq!(page("<p>x</p>").header("x-robots-tag", "noindex").status("crawlability/page-indexable"), Fail);
        let c = page(r#"<link rel="canonical" href="/good">"#);
        assert_eq!(c.results("crawlability/canonical-valid")[0].evidence.as_deref(), Some("https://example.com/good"));
        assert_eq!(page("<p>x</p>").status("crawlability/canonical-valid"), Warn);
        assert_eq!(page(r#"<link rel="canonical" href="http://[bad">"#).status("crawlability/canonical-valid"), Fail);
        assert_eq!(
            page("<p>x</p>").header("x-robots-tag", "NoFollow").message("crawlability/x-robots-tag"),
            "X-Robots-Tag: nofollow"
        );
        assert_eq!(page("<p>x</p>").header("x-robots-tag", "all").status("crawlability/x-robots-tag"), Pass);
    }
}
