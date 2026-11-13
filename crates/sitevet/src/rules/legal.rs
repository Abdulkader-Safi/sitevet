//! legal rules, ported from the TS package's `src/rules/legal.ts`.

use std::sync::Arc;

use super::kit::re;
use crate::{Link, Rule, RuleResult as R};

const CATEGORY: &str = "legal";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(PrivacyPolicy),
        Arc::new(Terms),
        Arc::new(CookieConsent),
        Arc::new(Contact),
        Arc::new(RefundPolicy),
        Arc::new(AdDisclosure),
    ]
}

/// A link whose text or raw href matches.
pub(crate) fn link_matches(links: &[Link], pattern: &str) -> bool {
    let r = re(pattern);
    links.iter().any(|l| r.is_match(&l.text) || r.is_match(&l.href))
}

dom_rule!(
    PrivacyPolicy,
    "legal/privacy-policy",
    "Privacy policy link",
    Warning,
    "Link to a privacy policy. Most jurisdictions require one if you collect any personal data.",
    |cx, dom, html| {
        if link_matches(&cx.page.links, "(?i)privacy") {
            vec![R::pass("Privacy policy link found")]
        } else {
            vec![R::warn("No privacy policy link found")]
        }
    }
);

dom_rule!(
    Terms,
    "legal/terms",
    "Terms of service link",
    Info,
    "Link to terms of service or terms and conditions to set the rules for using your site.",
    |cx, dom, html| {
        if link_matches(&cx.page.links, "(?i)terms|conditions") {
            vec![R::pass("Terms link found")]
        } else {
            vec![R::info("No terms of service link found")]
        }
    }
);

dom_rule!(
    CookieConsent,
    "legal/cookie-consent",
    "Cookie consent",
    Info,
    "If you set non-essential cookies, show a consent banner (a CMP) to comply with privacy law.",
    |cx, dom, html| {
        if re(r"(?i)cookieconsent|cookiebot|onetrust|osano|termly|usercentrics|cookie-?banner|cookie consent")
            .is_match(html)
        {
            vec![R::pass("Cookie consent mechanism detected")]
        } else {
            vec![R::info("No cookie consent banner detected")]
        }
    }
);

dom_rule!(
    Contact,
    "legal/contact",
    "Contact information",
    Info,
    "Provide a contact link or page. It builds trust and is a positive E-E-A-T signal.",
    |cx, dom, html| {
        if link_matches(&cx.page.links, "(?i)contact|mailto:") {
            vec![R::pass("Contact link found")]
        } else {
            vec![R::info("No contact link found")]
        }
    }
);

dom_rule!(
    RefundPolicy,
    "legal/refund-policy",
    "Refund or returns page",
    Info,
    "If you sell anything, link to a refund or returns policy from every page, usually in the footer.",
    |cx, dom, html| {
        if link_matches(&cx.page.links, crate::trust::REFUND) {
            vec![R::pass("Refund or returns page link found")]
        } else {
            vec![R::info("No refund or returns page link found")]
        }
    }
);

dom_rule!(
    AdDisclosure,
    "legal/ad-disclosure",
    "Ads or affiliate disclosure",
    Info,
    "Pages with affiliate links or ads should say so near the top, for example \"This post contains affiliate links\".",
    |cx, dom, html| {
        match crate::trust::disclosure(cx, html) {
            crate::trust::Disclosure::Disclosed => vec![R::pass("Ads or affiliate disclosure found")],
            crate::trust::Disclosure::Missing => vec![R::warn("Affiliate links or ads without a disclosure")],
            crate::trust::Disclosure::NotNeeded => vec![R::info("No affiliate links or ads detected")],
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Info, Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn legal_links() {
        let p = page(
            r#"<a href="/privacy">Privacy</a><a href="/t">Terms of use</a><a href="mailto:a@b.c">Email</a><div class="cookie-banner"></div>"#,
        );
        for id in ["privacy-policy", "terms", "cookie-consent", "contact"] {
            assert_eq!(p.status(&format!("legal/{id}")), Pass, "{id}");
        }
        let none = page("<p>x</p>");
        assert_eq!(none.status("legal/privacy-policy"), Warn);
        for id in ["terms", "cookie-consent", "contact"] {
            assert_eq!(none.status(&format!("legal/{id}")), Info, "{id}");
        }
    }
}
