//! What the page told crawlers, from the TS package's `src/metrics/directives.ts`.

use std::sync::Arc;

use url::Url;

use super::{cell, num, txt, value};
use crate::report::ValueStatus;
use crate::rules::HREFLANG;
use crate::rules::kit::{all, attr_trim, clip, header, re};
use crate::{Collector, MetricInput};

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(Directives), Arc::new(Hreflang)]
}

/// Same page, ignoring host case, a trailing slash and the fragment.
fn same_url(a: &str, b: &str) -> bool {
    let norm = |u: &str| match Url::parse(u) {
        Ok(p) => {
            let host = p.host_str().map(|h| match p.port() {
                Some(port) => format!("{h}:{port}"),
                None => h.to_string(),
            });
            let search = p.query().filter(|q| !q.is_empty()).map(|q| format!("?{q}")).unwrap_or_default();
            format!(
                "{}://{}{}{search}",
                p.scheme(),
                host.unwrap_or_default().to_lowercase(),
                p.path().strip_suffix('/').unwrap_or(p.path())
            )
        }
        Err(_) => u.to_string(),
    };
    norm(a) == norm(b)
}

fn resolve(href: &str, base: &str) -> String {
    Url::parse(base).and_then(|b| b.join(href)).map_or_else(|_| href.to_string(), String::from)
}

dom_collector!(Directives, "crawlability", "crawlability/directives", |cx, dom, html| {
    let robots_meta = attr_trim(dom, r#"meta[name="robots"]"#, "content");
    let x_robots = header(cx.page, "x-robots-tag").map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let canonical = attr_trim(dom, r#"link[rel="canonical"]"#, "href").map(|c| resolve(&c, &cx.page.final_url));
    let noindex = re("(?i)noindex");
    let blocked =
        noindex.is_match(&format!("{} {}", robots_meta.as_deref().unwrap_or(""), x_robots.as_deref().unwrap_or("")));
    let robots = &cx.site.robots;
    let directive = re(r"(?i)^\s*[a-z-]+\s*:");
    let directives =
        if robots.exists { robots.content.split('\n').filter(|l| directive.is_match(l)).count() } else { 0 };
    let matches = canonical.as_deref().map(|c| same_url(c, &cx.page.final_url));
    let present = |p: bool| if p { ValueStatus::Ok } else { ValueStatus::Missing };

    vec![
        value("crawlability/robots-meta", "Robots meta", robots_meta.clone().and_then(txt))
            .with_status(match &robots_meta {
                Some(_) if blocked => ValueStatus::Bad,
                Some(_) => ValueStatus::Ok,
                None => ValueStatus::Missing,
            })
            .with_note("Absent means index, follow"),
        value("crawlability/x-robots-tag", "X-Robots-Tag", x_robots.clone().and_then(txt))
            .with_status(match &x_robots {
                Some(x) if noindex.is_match(x) => ValueStatus::Bad,
                Some(_) => ValueStatus::Ok,
                None => ValueStatus::Missing,
            })
            .with_note("The header form of the same directive"),
        value("crawlability/canonical-url", "Canonical URL", canonical.clone().and_then(txt))
            .with_status(present(canonical.is_some())),
        value(
            "crawlability/canonical-match",
            "Canonical matches this URL",
            matches.and_then(|m| txt(if m { "yes" } else { "no" })),
        )
        .with_status(match matches {
            Some(true) => ValueStatus::Ok,
            Some(false) => ValueStatus::Warn,
            None => ValueStatus::Missing,
        })
        .with_note(if matches == Some(false) { cx.page.final_url.clone() } else { String::new() }),
        value(
            "crawlability/robots-txt",
            "robots.txt",
            if robots.exists { txt(format!("{directives} directives")) } else { None },
        )
        .with_status(present(robots.exists))
        .with_note(if robots.sitemaps.is_empty() {
            "No Sitemap line".to_string()
        } else {
            format!("Declares {} sitemap(s)", robots.sitemaps.len())
        }),
        value(
            "crawlability/sitemap-urls",
            "Sitemap URLs",
            if cx.site.sitemap.exists { num(cx.site.sitemap.urls.len() as f64) } else { None },
        )
        .with_status(present(cx.site.sitemap.exists)),
    ]
});

dom_collector!(Hreflang, "internationalization", "internationalization/hreflang", |cx, dom, html| {
    let mut self_ref = false;
    let rows: Vec<_> = all(dom, HREFLANG)
        .into_iter()
        .map(|el| {
            let lang = el.value().attr("hreflang").unwrap_or("").trim().to_string();
            let href = el.value().attr("href").unwrap_or("").trim();
            let resolved = resolve(href, &cx.page.final_url);
            self_ref |= same_url(&resolved, &cx.page.final_url);
            vec![cell(lang), cell(clip(&resolved, 90))]
        })
        .collect();
    let n = rows.len();
    let count = value("internationalization/hreflang-count", "hreflang alternates", num(n as f64))
        .with_status(match (n, self_ref) {
            (0, _) => ValueStatus::Missing,
            (_, true) => ValueStatus::Ok,
            _ => ValueStatus::Warn,
        })
        .with_note(match (n, self_ref) {
            (0, _) => "",
            (_, true) => "Includes a self-reference",
            _ => "No self-reference",
        });
    // No alternates, no table: the count already says it.
    if n == 0 {
        return vec![count];
    }
    vec![
        count,
        MetricInput::table("internationalization/hreflang-table", "hreflang alternates", &["hreflang", "URL"], rows),
    ]
});

#[cfg(test)]
mod tests {
    #[test]
    fn same_url_ignores_noise() {
        assert!(super::same_url("https://EXAMPLE.com/good/", "https://example.com/good#x"));
        assert!(!super::same_url("https://example.com/a?x=1", "https://example.com/a"));
    }
}
