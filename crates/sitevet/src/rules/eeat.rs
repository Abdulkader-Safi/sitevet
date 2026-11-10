//! eeat rules, ported from the TS package's `src/rules/eeat.ts`: the static subset of
//! experience, expertise, authority and trust signals.

use std::sync::Arc;

use super::kit::{attr, count, re};
use super::links::is_http;
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "eeat";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![Arc::new(Author), Arc::new(Date), Arc::new(OutboundCitations), Arc::new(AboutPage)]
}

dom_rule!(
    Author,
    "eeat/author",
    "Author attribution",
    Info,
    "Attribute content to a named author via a byline, rel=author, or schema author property.",
    |cx, dom, html| {
        let has = attr(dom, r#"meta[name="author"]"#, "content").is_some_and(|a| !a.is_empty())
            || count(dom, r#"[rel="author"]"#) > 0
            || re(r#""author"\s*:"#).is_match(html);
        if has { vec![R::pass("Author attribution present")] } else { vec![R::warn("No author attribution found")] }
    }
);

dom_rule!(
    Date,
    "eeat/date",
    "Published or updated date",
    Info,
    "Show a published or last-updated date. Freshness and transparency are trust signals.",
    |cx, dom, html| {
        let has = count(dom, r#"meta[property="article:published_time"]"#) > 0
            || count(dom, r#"meta[property="article:modified_time"]"#) > 0
            || count(dom, "time[datetime]") > 0
            || re(r#""date(Published|Modified)"\s*:"#).is_match(html);
        if has {
            vec![R::pass("Date information present")]
        } else {
            vec![R::info("No published or updated date found")]
        }
    }
);

dom_rule!(
    OutboundCitations,
    "eeat/outbound-citations",
    "Outbound citations",
    Info,
    "Cite authoritative external sources. Outbound links to reputable sites support your claims.",
    |cx, dom, html| {
        let external = cx.page.links.iter().filter(|l| !l.internal && l.url.as_deref().is_some_and(is_http)).count();
        if external > 0 {
            vec![R::pass(format!("{external} outbound link(s)"))]
        } else {
            vec![R::info("No outbound citations")]
        }
    }
);

dom_rule!(
    AboutPage,
    "eeat/about-page",
    "About page",
    Info,
    "Link to an about page that explains who is behind the site. It is a core trust signal.",
    |cx, dom, html| {
        let (text, href) = (re("(?i)about"), re("(?i)/about"));
        if cx.page.links.iter().any(|l| text.is_match(&l.text) || href.is_match(&l.href)) {
            vec![R::pass("About page link found")]
        } else {
            vec![R::info("No about page link found")]
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Info, Pass, Warn};
    use crate::rules::testkit::{fixture, page};

    #[test]
    fn eeat_signals() {
        let dated = page(&fixture("dated.html"));
        assert_eq!(dated.status("eeat/author"), Pass, "author in JSON-LD");
        assert_eq!(dated.status("eeat/date"), Pass);
        let none = page("<p>x</p>");
        assert_eq!(none.status("eeat/author"), Warn);
        assert_eq!(none.status("eeat/date"), Info);
        assert_eq!(page(r#"<time datetime="2026-01-01">Jan</time>"#).status("eeat/date"), Pass);
        let links = page(r#"<a href="https://who.int/x">WHO</a><a href="/team">About us</a>"#);
        assert_eq!(links.message("eeat/outbound-citations"), "1 outbound link(s)");
        assert_eq!(links.status("eeat/about-page"), Pass);
        assert_eq!(none.status("eeat/outbound-citations"), Info);
        assert_eq!(none.status("eeat/about-page"), Info);
    }
}
