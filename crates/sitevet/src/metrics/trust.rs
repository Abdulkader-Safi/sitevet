//! Trust and local SEO measurements (FR-TRUST-01, 02, 04 and 05): the author byline and dates as
//! values, the structured data trust signals, citation quality and LocalBusiness completeness.

use std::sync::Arc;

use super::{cell, num, txt, value};
use crate::report::ValueStatus;
use crate::{Collector, MetricInput};

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(Trust), Arc::new(LocalSeo)]
}

fn yes_no(b: bool) -> crate::MetricCell {
    cell(if b { "yes" } else { "no" })
}

fn present(b: bool) -> ValueStatus {
    if b { ValueStatus::Ok } else { ValueStatus::Missing }
}

dom_collector!(Trust, "eeat", "eeat/trust", |cx, dom, html| {
    let by = crate::trust::byline(cx, dom);
    let (published, updated) = crate::trust::dates(cx, dom);
    let (good, total) = crate::trust::citations(cx);
    let share = (total > 0).then(|| (good as f64 * 1000.0 / total as f64).round() / 10.0);
    vec![
        value("eeat/author-name", "Author", by.name.clone().and_then(txt))
            .with_status(present(by.name.is_some()))
            .with_note(if by.unit { "Byline or author box on the page" } else { "From metadata only, no byline on the page" }),
        value("eeat/author-link", "Author page link", by.link.clone().and_then(txt)).with_status(present(by.link.is_some())),
        value("eeat/author-bio", "Author bio", txt(if by.bio { "yes" } else { "no" })).with_status(present(by.bio)),
        value("eeat/date-published", "Published", published.clone().and_then(txt)).with_status(present(published.is_some())),
        value("eeat/date-updated", "Updated", updated.clone().and_then(txt)).with_status(present(updated.is_some())),
        MetricInput::table(
            "eeat/schema-trust-signals",
            "Structured data trust signals",
            &["Signal", "Present"],
            crate::trust::schema_signals(cx).iter().map(|(name, ok)| vec![cell(*name), yes_no(*ok)]).collect(),
        ),
        MetricInput::value("eeat/citation-quality", "Authoritative citations", share.and_then(num), Some("%"))
            .with_status(match share {
                None => ValueStatus::Missing,
                Some(s) if s > 0.0 => ValueStatus::Ok,
                Some(_) => ValueStatus::Warn,
            })
            .with_note(format!(
                "{good} of {total} outbound links in the main content go to government, education, standards or reference sites"
            )),
    ]
});

dom_collector!(LocalSeo, "eeat", "eeat/local-seo", |cx, dom, html| {
    // Only pages with LocalBusiness schema get these: other pages are not local listings.
    let Some(business) = crate::trust::local_business(cx) else { return Vec::new() };
    let checks = [
        ("Address", business.has("address")),
        ("Geo coordinates", business.has("geo")),
        ("Opening hours", business.has("openingHours") || business.has("openingHoursSpecification")),
        ("Telephone", business.has("telephone")),
    ];
    let profile = crate::trust::business_profile_link(cx);
    let missing: Vec<&str> = checks.iter().filter(|(_, ok)| !ok).map(|(n, _)| *n).collect();
    vec![
        MetricInput::table(
            "eeat/local-business-fields",
            "LocalBusiness schema",
            &["Field", "Present"],
            checks.iter().map(|(n, ok)| vec![cell(*n), yes_no(*ok)]).collect(),
        )
        .with_note(if missing.is_empty() {
            "Complete".to_string()
        } else {
            format!("Missing: {}", missing.join(", "))
        }),
        value("eeat/business-profile-link", "Google Business Profile link", profile.clone().and_then(txt))
            .with_status(present(profile.is_some()))
            .with_note("A link to the business on Google Maps or its Business Profile"),
    ]
});

#[cfg(test)]
mod tests {
    use crate::Status;
    use crate::report::ValueStatus::{Missing, Ok};
    use crate::rules::testkit::{Page, fixture, n, page, row, rows, t, value};

    fn v(p: &Page, id: &str) -> Option<crate::MetricCell> {
        value(&p.metric(id)).0
    }

    #[test]
    fn article_byline_dates_signals_and_citations() {
        let p = page(&fixture("trust.html"));
        let author = value(&p.metric("eeat/author-name"));
        assert_eq!((author.0, author.2.as_deref()), (t("Ana Lopez"), Some("Byline or author box on the page")));
        assert_eq!(v(&p, "eeat/author-link"), t("/team/ana"));
        assert_eq!(v(&p, "eeat/author-bio"), t("yes"));
        assert_eq!(v(&p, "eeat/date-published"), t("2026-03-02"));
        assert_eq!(v(&p, "eeat/date-updated"), t("2026-09-14"));
        assert_eq!(
            rows(&p.metric("eeat/schema-trust-signals")),
            [
                row(&["Article", "yes"]),
                row(&["BlogPosting", "no"]),
                row(&["Organization", "yes"]),
                row(&["author", "yes"]),
                row(&["datePublished", "yes"]),
                row(&["dateModified", "yes"]),
            ]
        );
        // FR-TRUST-04: who.int and a blog.
        let citations = value(&p.metric("eeat/citation-quality"));
        assert_eq!((citations.0, citations.1), (n(50.0), Some(Ok)));
        // No LocalBusiness: no local SEO tables.
        assert!(p.metrics().iter().all(|m| m.id != "eeat/local-business-fields"));
    }

    #[test]
    fn trust_pages_and_contact_details() {
        let p = page(&fixture("trust.html"));
        assert_eq!(p.status("legal/refund-policy"), Status::Pass);
        assert_eq!(p.status("eeat/editorial-policy"), Status::Pass);
        assert_eq!(p.message("eeat/contact-details"), "Phone number and address found");
        assert_eq!(p.message("legal/ad-disclosure"), "No affiliate links or ads detected");
        assert_eq!(p.status("eeat/nap-consistency"), Status::Info);
        let bare = page("<p>Hello</p>");
        for id in ["legal/refund-policy", "eeat/editorial-policy", "eeat/contact-details", "legal/ad-disclosure"] {
            assert_eq!(bare.status(id), Status::Info, "{id}");
        }
        // Other languages: a French returns page.
        assert_eq!(
            page(r#"<a href="/retours">Retours et remboursement</a>"#).status("legal/refund-policy"),
            Status::Pass
        );
        let affiliate = r#"<a href="https://amzn.to/xyz">Buy the spade</a>"#;
        assert_eq!(page(affiliate).message("legal/ad-disclosure"), "Affiliate links or ads without a disclosure");
        let disclosed = format!("<p>This post contains affiliate links.</p>{affiliate}");
        assert_eq!(page(&disclosed).status("legal/ad-disclosure"), Status::Pass);
    }

    #[test]
    fn local_business_nap_and_profile() {
        let p = page(&fixture("local.html"));
        assert_eq!(p.status("eeat/nap-consistency"), Status::Warn);
        assert_eq!(
            p.message("eeat/nap-consistency"),
            "NAP mismatch: phone on the page (96522229999) differs from the schema (+965 2222 1111)."
        );
        assert_eq!(
            rows(&p.metric("eeat/local-business-fields")),
            [
                row(&["Address", "yes"]),
                row(&["Geo coordinates", "no"]),
                row(&["Opening hours", "yes"]),
                row(&["Telephone", "yes"]),
            ]
        );
        let profile = value(&p.metric("eeat/business-profile-link"));
        assert_eq!((profile.0, profile.1), (t("https://maps.app.goo.gl/abc123"), Some(Ok)));
        let fixed = page(&fixture("local.html").replace("2222 9999", "2222 1111"));
        assert_eq!(fixed.message("eeat/nap-consistency"), "Phone, name and street address match the page text.");
        // A schema with only a name says what it could not check.
        let name_only = page(
            r#"<script type="application/ld+json">{"@type":"ProfessionalService","name":"Studio"}</script><p>Studio</p>"#,
        );
        assert_eq!(
            name_only.message("eeat/nap-consistency"),
            "Name matches the page text. The schema has no street address or phone to compare."
        );
        // An address only in JSON-LD is not on the page.
        let ld_only = page(
            r#"<script type="application/ld+json">{"@type":"Organization","address":{"@type":"PostalAddress","streetAddress":"1 Main Street"}}</script><p>x</p>"#,
        );
        assert_eq!(ld_only.status("eeat/contact-details"), Status::Info);
        assert_eq!(value(&page("<p>x</p>").metric("eeat/citation-quality")).1, Some(Missing));
    }
}
