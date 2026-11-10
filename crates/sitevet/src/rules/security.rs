//! security rules, ported from the TS package's `src/rules/security.ts`.

use std::sync::Arc;

use super::kit::header;
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "security";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(ServerDisclosure),
        Arc::new(XPoweredBy),
        Arc::new(CookieSecure),
        Arc::new(Coop),
        Arc::new(Https),
        Arc::new(Hsts),
        Arc::new(Csp),
        Arc::new(XFrameOptions),
        Arc::new(XContentTypeOptions),
        Arc::new(ReferrerPolicy),
        Arc::new(MixedContent),
        Arc::new(PermissionsPolicy),
    ]
}

/// A header that is present and not empty, as JavaScript's truthiness reads it.
fn set(page: &crate::PageData, name: &str) -> Option<String> {
    header(page, name).filter(|v| !v.is_empty())
}

dom_rule!(
    ServerDisclosure,
    "security/server-disclosure",
    "Server header disclosure",
    Info,
    "Remove or genericise the Server header so it does not reveal software and version.",
    |cx, dom, html| {
        match set(cx.page, "server") {
            Some(s) if s.chars().any(|c| c.is_ascii_digit()) => {
                vec![R::warn(format!("Server header reveals a version: {s}")).with_evidence(s)]
            }
            _ => vec![R::pass("Server header does not leak a version")],
        }
    }
);

dom_rule!(
    XPoweredBy,
    "security/x-powered-by",
    "X-Powered-By disclosure",
    Info,
    "Remove the X-Powered-By header. It discloses your stack and helps attackers.",
    |cx, dom, html| {
        match set(cx.page, "x-powered-by") {
            Some(p) => vec![R::warn(format!("X-Powered-By exposed: {p}")).with_evidence(p)],
            None => vec![R::pass("No X-Powered-By header")],
        }
    }
);

dom_rule!(
    CookieSecure,
    "security/cookie-secure",
    "Secure cookies",
    Warning,
    "Set Secure and HttpOnly on cookies so they are not sent over HTTP or read by scripts.",
    |cx, dom, html| {
        let Some(c) = set(cx.page, "set-cookie") else {
            return vec![R::pass("No cookies set on this response")];
        };
        let lower = c.to_lowercase();
        let missing: Vec<&str> = [("secure", "Secure"), ("httponly", "HttpOnly")]
            .into_iter()
            .filter(|(needle, _)| !lower.contains(needle))
            .map(|(_, name)| name)
            .collect();
        if missing.is_empty() {
            vec![R::pass("Cookies set with Secure and HttpOnly")]
        } else {
            vec![R::warn(format!("Cookie missing {}", missing.join(" and ")))]
        }
    }
);

dom_rule!(
    Coop,
    "security/coop",
    "Cross-Origin-Opener-Policy",
    Info,
    "Add Cross-Origin-Opener-Policy: same-origin to isolate your browsing context.",
    |cx, dom, html| {
        match set(cx.page, "cross-origin-opener-policy") {
            Some(_) => vec![R::pass("COOP set")],
            None => vec![R::info("No Cross-Origin-Opener-Policy header")],
        }
    }
);

dom_rule!(
    Https,
    "security/https",
    "Served over HTTPS",
    Error,
    "Serve the site over HTTPS and redirect all HTTP traffic to HTTPS.",
    |cx, dom, html| {
        if cx.page.final_url.starts_with("https://") {
            vec![R::pass("HTTPS")]
        } else {
            vec![R::fail("Not served over HTTPS")]
        }
    }
);

dom_rule!(
    Hsts,
    "security/hsts",
    "HSTS header",
    Warning,
    "Add the Strict-Transport-Security header to enforce HTTPS on future visits.",
    |cx, dom, html| {
        match set(cx.page, "strict-transport-security") {
            Some(_) => vec![R::pass("HSTS set")],
            None => vec![R::warn("Missing Strict-Transport-Security")],
        }
    }
);

dom_rule!(
    Csp,
    "security/csp",
    "Content-Security-Policy",
    Warning,
    "Add a Content-Security-Policy header to limit what scripts and resources can load.",
    |cx, dom, html| {
        match set(cx.page, "content-security-policy") {
            Some(_) => vec![R::pass("CSP set")],
            None => vec![R::warn("Missing Content-Security-Policy")],
        }
    }
);

dom_rule!(
    XFrameOptions,
    "security/x-frame-options",
    "Clickjacking protection",
    Warning,
    "Set X-Frame-Options: DENY or a CSP frame-ancestors directive to block clickjacking.",
    |cx, dom, html| {
        let csp = header(cx.page, "content-security-policy").unwrap_or_default();
        if set(cx.page, "x-frame-options").is_some() || csp.contains("frame-ancestors") {
            vec![R::pass("Clickjacking protection set")]
        } else {
            vec![R::warn("No X-Frame-Options or frame-ancestors")]
        }
    }
);

dom_rule!(
    XContentTypeOptions,
    "security/x-content-type-options",
    "X-Content-Type-Options",
    Warning,
    "Add X-Content-Type-Options: nosniff to stop MIME-type sniffing.",
    |cx, dom, html| {
        if header(cx.page, "x-content-type-options").unwrap_or_default().to_lowercase() == "nosniff" {
            vec![R::pass("nosniff set")]
        } else {
            vec![R::warn("Missing X-Content-Type-Options: nosniff")]
        }
    }
);

dom_rule!(
    ReferrerPolicy,
    "security/referrer-policy",
    "Referrer-Policy",
    Info,
    "Add a Referrer-Policy header, e.g. strict-origin-when-cross-origin.",
    |cx, dom, html| {
        match set(cx.page, "referrer-policy") {
            Some(_) => vec![R::pass("Referrer-Policy set")],
            None => vec![R::warn("Missing Referrer-Policy")],
        }
    }
);

dom_rule!(
    MixedContent,
    "security/mixed-content",
    "Mixed content",
    Error,
    "Load every resource over HTTPS. Replace any http:// references with https://.",
    |cx, dom, html| {
        if !cx.page.final_url.starts_with("https://") {
            return vec![R::pass("Not HTTPS, mixed content N/A")];
        }
        let links = cx.page.links.iter().filter_map(|l| l.url.as_deref());
        let images = cx.page.images.iter().filter_map(|i| i.url.as_deref());
        let insecure = links.chain(images).filter(|u| u.starts_with("http://")).count();
        if insecure == 0 {
            vec![R::pass("No mixed content")]
        } else {
            vec![R::fail(format!("{insecure} insecure http resource(s) on HTTPS page"))]
        }
    }
);

dom_rule!(
    PermissionsPolicy,
    "security/permissions-policy",
    "Permissions-Policy",
    Info,
    "Add a Permissions-Policy header to restrict powerful browser features the site does not use.",
    |cx, dom, html| {
        match set(cx.page, "permissions-policy") {
            Some(_) => vec![R::pass("Permissions-Policy set")],
            None => vec![R::info("No Permissions-Policy header")],
        }
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{Fail, Info, Pass, Warn};
    use crate::rules::testkit::page;

    #[test]
    fn headers_present_and_missing() {
        let all = page("<p>x</p>")
            .header("strict-transport-security", "max-age=1")
            .header("content-security-policy", "default-src 'self'")
            .header("x-frame-options", "DENY")
            .header("x-content-type-options", "nosniff")
            .header("referrer-policy", "no-referrer")
            .header("permissions-policy", "camera=()")
            .header("cross-origin-opener-policy", "same-origin")
            .header("server", "nginx");
        let none = page("<p>x</p>").header("server", "nginx/1.25.3").header("x-powered-by", "PHP/8");
        for id in [
            "hsts",
            "csp",
            "x-frame-options",
            "x-content-type-options",
            "referrer-policy",
            "server-disclosure",
            "x-powered-by",
        ] {
            assert_eq!(all.status(&format!("security/{id}")), Pass, "{id}");
            assert_eq!(none.status(&format!("security/{id}")), Warn, "{id}");
        }
        for id in ["permissions-policy", "coop"] {
            assert_eq!(all.status(&format!("security/{id}")), Pass, "{id}");
            assert_eq!(none.status(&format!("security/{id}")), Info, "{id}");
        }
        let frame_ancestors = page("<p>x</p>").header("content-security-policy", "frame-ancestors 'none'");
        assert_eq!(frame_ancestors.status("security/x-frame-options"), Pass);
    }

    #[test]
    fn cookies() {
        assert_eq!(page("<p>x</p>").status("security/cookie-secure"), Pass);
        let ok = page("<p>x</p>").header("set-cookie", "a=1; Secure; HttpOnly");
        assert_eq!(ok.status("security/cookie-secure"), Pass);
        let bad = page("<p>x</p>").header("set-cookie", "a=1");
        assert_eq!(bad.message("security/cookie-secure"), "Cookie missing Secure and HttpOnly");
    }

    #[test]
    fn https_and_mixed_content() {
        let secure = page(r#"<a href="https://o.test/">o</a><img src="/a.png">"#);
        assert_eq!(secure.status("security/https"), Pass);
        assert_eq!(secure.status("security/mixed-content"), Pass);
        let mixed = page(r#"<a href="http://o.test/">o</a><img src="http://cdn.test/a.png">"#);
        assert_eq!(mixed.message("security/mixed-content"), "2 insecure http resource(s) on HTTPS page");
        let plain = page("<p>x</p>").url("http://example.com/x");
        assert_eq!(plain.status("security/https"), Fail);
        assert_eq!(plain.message("security/mixed-content"), "Not HTTPS, mixed content N/A");
    }
}
