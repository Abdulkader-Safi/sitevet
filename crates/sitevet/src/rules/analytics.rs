//! analytics rules, ported from the TS package's `src/rules/analytics.ts`.

use std::sync::Arc;

use super::kit::re;
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "analytics";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![Arc::new(TagPresent), Arc::new(ConsentMode)]
}

dom_rule!(
    TagPresent,
    "analytics/tag-present",
    "Analytics installed",
    Info,
    "Install an analytics tag (GA4, Plausible, Fathom, etc.) so you can measure traffic and behaviour.",
    |cx, dom, html| {
        let hints = re(
            r"(?i)(googletagmanager\.com|gtag\(|google-analytics\.com|ga\.js|analytics\.js|plausible\.io|cdn\.usefathom|matomo|segment\.com/analytics|posthog|mixpanel|hotjar|clarity\.ms)",
        );
        if hints.is_match(html) {
            vec![R::pass("Analytics tag detected")]
        } else {
            vec![R::warn("No analytics tag detected")]
        }
    }
);

dom_rule!(
    ConsentMode,
    "analytics/consent-mode",
    "Consent mode",
    Info,
    "If you use Google tags in regulated regions, configure Consent Mode so tracking respects user consent.",
    |cx, dom, html| {
        if !re(r"(?i)googletagmanager\.com|gtag\(").is_match(html) {
            return vec![R::pass("No Google tag, consent mode not required")];
        }
        if re(r#"(?i)consent['"\s,]+(default|update)"#).is_match(html) {
            vec![R::pass("Google Consent Mode configured")]
        } else {
            vec![R::info("Google tag present without visible Consent Mode")]
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Info, Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn tags_and_consent() {
        let gtag = r#"<script src="https://www.googletagmanager.com/gtag/js?id=G-1"></script>"#;
        assert_eq!(page(gtag).status("analytics/tag-present"), Pass);
        assert_eq!(page("<p>x</p>").status("analytics/tag-present"), Warn);
        assert_eq!(page("<p>x</p>").status("analytics/consent-mode"), Pass);
        assert_eq!(page(gtag).status("analytics/consent-mode"), Info);
        let consent = format!("{gtag}<script>gtag('consent', 'default', {{}});</script>");
        assert_eq!(page(&consent).message("analytics/consent-mode"), "Google Consent Mode configured");
    }
}
