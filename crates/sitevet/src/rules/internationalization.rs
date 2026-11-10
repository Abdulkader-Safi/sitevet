//! internationalization rules, ported from the TS package's `src/rules/internationalization.ts`.

use std::sync::Arc;

use super::kit::{all, re};
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "internationalization";
pub(crate) const HREFLANG: &str = r#"link[rel="alternate"][hreflang]"#;

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![Arc::new(HreflangPresent), Arc::new(HreflangValid)]
}

dom_rule!(
    HreflangPresent,
    "internationalization/hreflang-present",
    "hreflang annotations",
    Info,
    "If the site serves more than one language or region, add hreflang link alternates so Google serves the right version.",
    |cx, dom, html| {
        match all(dom, HREFLANG).len() {
            0 => vec![R::info("No hreflang alternates (fine for a single-language site)")],
            n => vec![R::pass(format!("{n} hreflang alternate(s)"))],
        }
    }
);

dom_rule!(
    HreflangValid,
    "internationalization/hreflang-valid",
    "hreflang values valid",
    Warning,
    "Use valid hreflang values: a language code, language-REGION (en-US), or x-default. Include a self-referencing and an x-default tag.",
    |cx, dom, html| {
        let tags = all(dom, HREFLANG);
        if tags.is_empty() {
            return vec![R::pass("No hreflang to validate")];
        }
        let valid = re(r"^([a-z]{2,3}(-[A-Za-z]{2,4})?|x-default)$");
        let values: Vec<&str> = tags.iter().map(|t| t.value().attr("hreflang").unwrap_or("")).collect();
        let bad: Vec<&str> = values.iter().copied().filter(|v| !valid.is_match(v)).collect();
        if !bad.is_empty() {
            return vec![R::warn(format!("Invalid hreflang value(s): {}", bad.join(", ")))];
        }
        if !values.iter().any(|v| v.eq_ignore_ascii_case("x-default")) {
            return vec![R::warn("hreflang set has no x-default")];
        }
        vec![R::pass("hreflang values are valid and include x-default")]
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Info, Pass, Warn};
    use crate::rules::testkit::{fixture, page};

    #[test]
    fn hreflang() {
        let p = page(&fixture("directives.html"));
        assert_eq!(p.message("internationalization/hreflang-present"), "3 hreflang alternate(s)");
        assert_eq!(p.status("internationalization/hreflang-valid"), Pass);
        assert_eq!(page("<p>x</p>").status("internationalization/hreflang-present"), Info);
        let bad = page(
            r#"<link rel="alternate" hreflang="english" href="/en"><link rel="alternate" hreflang="x-default" href="/">"#,
        );
        assert_eq!(bad.message("internationalization/hreflang-valid"), "Invalid hreflang value(s): english");
        let no_default = page(r#"<link rel="alternate" hreflang="en-GB" href="/en">"#);
        assert_eq!(no_default.status("internationalization/hreflang-valid"), Warn);
    }
}
