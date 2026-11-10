//! links rules, ported from the TS package's `src/rules/links.ts`.

use std::sync::Arc;

use super::kit::re;
use crate::{Link, Rule, RuleCx, RuleResult as R, Status};

const CATEGORY: &str = "links";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(BrokenInternal),
        Arc::new(BrokenExternal),
        Arc::new(RedirectChains),
        Arc::new(WeakAnchorText),
        Arc::new(ExternalRel),
    ]
}

pub(crate) fn is_http(u: &str) -> bool {
    u.starts_with("http")
}

/// Distinct link URLs passing `keep`, in page order, and the ones the checker found broken.
fn broken(cx: &RuleCx<'_>, keep: impl Fn(&Link) -> bool) -> (usize, Vec<(String, u16)>) {
    let mut targets: Vec<&str> = Vec::new();
    for l in cx.page.links.iter().filter(|l| keep(l)) {
        if let Some(u) = l.url.as_deref()
            && !targets.contains(&u)
        {
            targets.push(u);
        }
    }
    let bad = targets
        .iter()
        // A link robots.txt keeps us from requesting has no known status, so it is not broken.
        .filter_map(|u| {
            cx.links
                .get(*u)
                .filter(|s| s.error.as_deref() != Some(crate::crawl::ROBOTS_BLOCKED))
                .map(|s| (u.to_string(), s.status))
        })
        .filter(|(_, status)| *status >= 400 || *status == 0)
        .collect();
    (targets.len(), bad)
}

fn status_text(status: u16) -> String {
    if status == 0 { "no response".into() } else { status.to_string() }
}

rule!(
    BrokenInternal,
    "links/broken-internal",
    "Broken internal links",
    Error,
    Enriched,
    "Fix or remove the linked internal URLs that return errors.",
    |cx| {
        let (n, bad) = broken(cx, |l| l.internal);
        if bad.is_empty() {
            return vec![R::pass(format!("All {n} internal links OK"))];
        }
        bad.into_iter()
            .map(|(u, s)| R::fail(format!("Broken internal link ({})", status_text(s))).with_evidence(u))
            .collect()
    }
);

rule!(
    BrokenExternal,
    "links/broken-external",
    "Broken external links",
    Warning,
    Enriched,
    "Update or remove external links that no longer resolve.",
    |cx| {
        let (n, bad) = broken(cx, |l| !l.internal && l.url.as_deref().is_some_and(is_http));
        if bad.is_empty() {
            return vec![R::pass(format!("All {n} external links OK"))];
        }
        bad.into_iter()
            .map(|(u, s)| R::new(Status::Warn, format!("Broken external link ({})", status_text(s))).with_evidence(u))
            .collect()
    }
);

dom_rule!(
    RedirectChains,
    "links/redirect-chains",
    "Redirect chains",
    Warning,
    "Point the link straight at the final URL so the page loads in one hop.",
    |cx, dom, html| {
        let hops = cx.page.redirect_chain.len();
        if hops > 1 {
            vec![R::warn(format!("Page redirected {hops} times before loading"))]
        } else {
            vec![R::pass("No redirect chain")]
        }
    }
);

dom_rule!(
    WeakAnchorText,
    "links/weak-anchor-text",
    "Weak anchor text",
    Info,
    "Replace generic anchors like 'click here' with text that describes the destination.",
    |cx, dom, html| {
        let weak = re(r"(?i)^(click here|read more|here|learn more|more|link|this)$");
        let bad: Vec<&Link> = cx.page.links.iter().filter(|l| weak.is_match(l.text.trim())).collect();
        if bad.is_empty() {
            return vec![R::pass("No generic anchor text")];
        }
        let sample: Vec<&str> = bad.iter().take(3).map(|l| l.text.as_str()).collect();
        vec![R::warn(format!("{} link(s) with generic anchor text", bad.len())).with_evidence(sample.join(", "))]
    }
);

dom_rule!(
    ExternalRel,
    "links/external-rel",
    "External link rel",
    Info,
    r#"Add rel="noopener" (and noreferrer) to external links, especially those opening new tabs."#,
    |cx, dom, html| {
        let missing = cx
            .page
            .links
            .iter()
            .filter(|l| {
                !l.internal
                    && l.url.as_deref().is_some_and(is_http)
                    && !l.rel.contains("noopener")
                    && !l.rel.contains("noreferrer")
            })
            .count();
        if missing == 0 {
            vec![R::pass("External links have safe rel attributes")]
        } else {
            vec![R::warn(format!("{missing} external link(s) missing rel=noopener"))]
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Fail, Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn broken_links() {
        let html = r#"<a href="/a">a</a><a href="/a#top">a again</a><a href="https://other.test/x">x</a><a href="mailto:a@b.c">m</a>"#;
        let ok = page(html);
        assert_eq!(ok.message("links/broken-internal"), "All 2 internal links OK");
        assert_eq!(ok.message("links/broken-external"), "All 1 external links OK");
        let mut bad = page(html);
        bad.links_ok = false;
        assert_eq!(bad.statuses("links/broken-internal"), [Fail, Fail]);
        assert_eq!(bad.results("links/broken-internal")[0].evidence.as_deref(), Some("https://example.com/a"));
        assert_eq!(bad.message("links/broken-external"), "Broken external link (404)");
        assert_eq!(bad.status("links/broken-external"), Warn);
        let mut down = page(html);
        down.link_status = vec![("https://other.test/x".into(), 0)];
        assert_eq!(down.message("links/broken-external"), "Broken external link (no response)");
    }

    #[test]
    fn redirect_chains() {
        let mut p = page("<p>x</p>");
        p.redirects = 1;
        assert_eq!(p.status("links/redirect-chains"), Pass);
        p.redirects = 2;
        assert_eq!(p.message("links/redirect-chains"), "Page redirected 2 times before loading");
    }

    #[test]
    fn anchors_and_rel() {
        let p = page(r#"<a href="/a"> Click here </a><a href="/b">Pricing</a>"#);
        assert_eq!(p.message("links/weak-anchor-text"), "1 link(s) with generic anchor text");
        assert_eq!(page(r#"<a href="/b">Pricing</a>"#).status("links/weak-anchor-text"), Pass);
        assert_eq!(page(r#"<a href="https://o.test/" rel="noopener">o</a>"#).status("links/external-rel"), Pass);
        assert_eq!(page(r#"<a href="https://o.test/">o</a><a href="/in">in</a>"#).status("links/external-rel"), Warn);
    }
}
