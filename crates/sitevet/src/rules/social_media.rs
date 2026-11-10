//! social-media rules, ported from the TS package's `src/rules/social-media.ts`.

use std::sync::Arc;

use super::kit::{attr, attr_trim, re};
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "social-media";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![Arc::new(TwitterCard), Arc::new(OgImageAbsolute), Arc::new(OgUrlCanonical), Arc::new(ProfileLinks)]
}

dom_rule!(
    TwitterCard,
    "social-media/twitter-card",
    "Twitter Card",
    Info,
    r#"Add <meta name="twitter:card" content="summary_large_image"> so links preview well on X/Twitter."#,
    |cx, dom, html| {
        match attr(dom, r#"meta[name="twitter:card"]"#, "content").filter(|c| !c.is_empty()) {
            Some(_) => vec![R::pass("Twitter Card present")],
            None => vec![R::warn("No twitter:card meta")],
        }
    }
);

dom_rule!(
    OgImageAbsolute,
    "social-media/og-image-absolute",
    "Open Graph image is absolute",
    Warning,
    "og:image must be an absolute https URL. Social platforms cannot resolve relative paths.",
    |cx, dom, html| {
        let Some(img) = attr_trim(dom, r#"meta[property="og:image"]"#, "content") else {
            return vec![R::warn("No og:image to validate")];
        };
        if re(r"(?i)^https?://").is_match(&img) {
            vec![R::pass("og:image is an absolute URL")]
        } else {
            vec![R::fail("og:image is not an absolute URL").with_evidence(img)]
        }
    }
);

dom_rule!(
    OgUrlCanonical,
    "social-media/og-url-canonical",
    "og:url matches canonical",
    Info,
    "Set og:url to the page's canonical URL so shares consolidate to one address.",
    |cx, dom, html| {
        let og = attr_trim(dom, r#"meta[property="og:url"]"#, "content");
        let canon = attr_trim(dom, r#"link[rel="canonical"]"#, "href");
        let (Some(og), Some(canon)) = (og, canon) else {
            return vec![R::pass("og:url or canonical absent, nothing to compare")];
        };
        let strip = |s: &str| s.strip_suffix('/').unwrap_or(s).to_string();
        if strip(&og) == strip(&canon) {
            vec![R::pass("og:url matches canonical")]
        } else {
            vec![R::warn("og:url does not match canonical").with_evidence(format!("{og} vs {canon}"))]
        }
    }
);

dom_rule!(
    ProfileLinks,
    "social-media/profile-links",
    "Social profile links",
    Info,
    "Link to your social profiles so visitors and search engines can connect your brand accounts.",
    |cx, dom, html| {
        let hosts = re(
            r"(?i)(linkedin\.com|twitter\.com|x\.com|instagram\.com|facebook\.com|youtube\.com|github\.com|tiktok\.com)",
        );
        if cx.page.links.iter().any(|l| hosts.is_match(l.url.as_deref().unwrap_or(""))) {
            vec![R::pass("Links to social profiles found")]
        } else {
            vec![R::info("No social profile links found")]
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Fail, Info, Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn cards_and_images() {
        assert_eq!(page(r#"<meta name="twitter:card" content="summary">"#).status("social-media/twitter-card"), Pass);
        assert_eq!(page("<p>x</p>").status("social-media/twitter-card"), Warn);
        assert_eq!(
            page(r#"<meta property="og:image" content="https://e.com/a.png">"#)
                .status("social-media/og-image-absolute"),
            Pass
        );
        assert_eq!(
            page(r#"<meta property="og:image" content="/a.png">"#).status("social-media/og-image-absolute"),
            Fail
        );
        assert_eq!(page("<p>x</p>").status("social-media/og-image-absolute"), Warn);
    }

    #[test]
    fn og_url_and_profiles() {
        let same =
            page(r#"<meta property="og:url" content="https://e.com/a/"><link rel="canonical" href="https://e.com/a">"#);
        assert_eq!(same.status("social-media/og-url-canonical"), Pass);
        let differ =
            page(r#"<meta property="og:url" content="https://e.com/b"><link rel="canonical" href="https://e.com/a">"#);
        assert_eq!(differ.status("social-media/og-url-canonical"), Warn);
        assert_eq!(page(r#"<a href="https://github.com/x">gh</a>"#).status("social-media/profile-links"), Pass);
        assert_eq!(page("<p>x</p>").status("social-media/profile-links"), Info);
    }
}
