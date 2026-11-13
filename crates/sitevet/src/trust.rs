//! Trust and local SEO signals (FR-TRUST-01 to FR-TRUST-05), shared by the `eeat` and `legal`
//! rules and the trust collectors so a rule and a measurement never disagree. The TS package
//! lists phase 5 in its backlog without code; the patterns here are sitevet's own.

use std::collections::HashSet;
use std::sync::LazyLock;

use scraper::{ElementRef, Html};

use crate::rules::kit::{all, attr, collapse, re, sel, text};
use crate::{RuleCx, SchemaNode, SchemaValue};

static PHONE: LazyLock<regex::Regex> = LazyLock::new(|| re(r"\+?\(?\d[\d\s().-]{6,}\d"));
static STREET: LazyLock<regex::Regex> = LazyLock::new(|| {
    re(r"(?i)\b\d{1,5}\s+(\w+\s){0,3}(street|st\.|road|rd\.|avenue|ave\.|boulevard|blvd|lane|drive|way|square|place)\b")
});

/// Refund or returns pages, in English and the common words of a few other languages.
pub(crate) const REFUND: &str = r"(?i)refund|returns?\b|return.policy|money.back|remboursement|retours?\b|rückgabe|erstattung|widerruf|devoluci[oó]n|reembolso|rimborso|resi\b|reso\b|terugbetaling|retouren|استرجاع|استرداد|إرجاع";
/// Editorial policy, ethics, corrections or review method pages.
pub(crate) const EDITORIAL: &str = r"(?i)editorial|ethics|corrections|fact.?check|our standards|how we (review|test)|ligne éditoriale|redaktion|política editorial|السياسة التحريرية";
/// Links that earn money: rel="sponsored", affiliate parameters and networks.
static AFFILIATE: LazyLock<regex::Regex> = LazyLock::new(|| {
    re(
        r"(?i)amzn\.to|amazon\.[a-z.]+/.*[?&]tag=|[?&](aff|affiliate|aff_id|affid)=|shareasale|awin1\.com|clickbank|anrdoezrs|jdoqocy|tkqlhce|impact\.com|partnerize|prf\.hn|go\.skimresources",
    )
});
static ADS: LazyLock<regex::Regex> =
    LazyLock::new(|| re(r"(?i)adsbygoogle|googlesyndication|doubleclick\.net|amazon-adsystem"));
static DISCLOSURE: LazyLock<regex::Regex> = LazyLock::new(|| {
    re(
        r"(?i)affiliate (link|links|commission|disclosure|program)|(we|i) (may )?earn (a )?(small )?commission|sponsored (post|content|by)|paid partnership|advertis(ing|er) disclosure|contains affiliate|#ad\b",
    )
});
/// Google Business Profile and Maps place links.
static BUSINESS_PROFILE: LazyLock<regex::Regex> = LazyLock::new(|| {
    re(
        r"(?i)^https?://(g\.page|maps\.app\.goo\.gl|business\.google\.com|g\.co/kgs|maps\.google\.[a-z.]+|(www\.)?google\.[a-z.]+/maps)",
    )
});

static AUTHORITATIVE: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    include_str!("../data/authoritative-domains.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
});

fn nodes<'a>(cx: &'a RuleCx<'_>) -> impl Iterator<Item = &'a SchemaNode> + 'a {
    cx.page.schema.iter().flat_map(|i| i.node.walk())
}

fn text_of(v: &SchemaValue) -> Option<String> {
    match v {
        SchemaValue::Text(t) => Some(t.trim().to_string()).filter(|t| !t.is_empty()),
        SchemaValue::Node(n) => n.values("name").find_map(text_of),
    }
}

/// The first value of a property on any node.
fn schema_value(cx: &RuleCx<'_>, property: &str) -> Option<String> {
    nodes(cx).find_map(|n| n.values(property).find_map(text_of))
}

/// FR-TRUST-01: an author byline read as one unit.
#[derive(Default)]
pub(crate) struct Byline {
    /// True when the page has an author box or byline element, not only metadata.
    pub(crate) unit: bool,
    pub(crate) name: Option<String>,
    pub(crate) link: Option<String>,
    pub(crate) bio: bool,
}

const AUTHOR_BOX: &str =
    "[class*=author i],[id*=author i],[class*=byline i],[id*=byline i],[rel~=author],[itemprop~=author]";

pub(crate) fn byline(cx: &RuleCx<'_>, dom: &Html) -> Byline {
    let boxes: Vec<ElementRef> = all(dom, AUTHOR_BOX)
        .into_iter()
        .filter(|e| !matches!(e.value().name(), "meta" | "link" | "script" | "style" | "body" | "html"))
        .collect();
    let outer = sel(AUTHOR_BOX);
    let unit = boxes.iter().copied().find(|e| !e.ancestors().filter_map(ElementRef::wrap).any(|a| outer.matches(&a)));
    let rel_author = all(dom, "a[rel~=author][href]").into_iter().next();
    let name = nodes(cx)
        .find_map(|n| n.values("author").find_map(text_of))
        .or_else(|| attr(dom, r#"meta[name="author"]"#, "content").filter(|a| !a.trim().is_empty()))
        .or_else(|| rel_author.map(|a| collapse(&text(a))).filter(|t| !t.is_empty()))
        .or_else(|| {
            let t = collapse(&text(unit?));
            let t = t.strip_prefix("By ").or_else(|| t.strip_prefix("by ")).unwrap_or(&t);
            let name: String = t.split([',', '|', '\u{b7}']).next().unwrap_or("").trim().chars().take(60).collect();
            Some(name).filter(|n| !n.is_empty())
        });
    let link = rel_author
        .and_then(|a| a.value().attr("href"))
        .or_else(|| unit.and_then(|u| u.select(&sel("a[href]")).next()).and_then(|a| a.value().attr("href")))
        .map(str::to_string)
        .or_else(|| nodes(cx).flat_map(|n| n.nodes("author")).find_map(|a| a.values("url").find_map(text_of)));
    let bio = !all(dom, "[class*=bio i],[id*=bio i],[itemprop~=description] [class*=author i]").is_empty()
        || unit.is_some_and(|u| crate::text::tokenize(&text(u)).len() >= 15);
    Byline { unit: unit.is_some(), name, link, bio }
}

/// Publish and updated dates as written: schema first, then meta tags, then `<time>`.
pub(crate) fn dates(cx: &RuleCx<'_>, dom: &Html) -> (Option<String>, Option<String>) {
    let meta = |p: &str| attr(dom, &format!(r#"meta[property="{p}"]"#), "content").filter(|v| !v.trim().is_empty());
    let published = schema_value(cx, "datePublished")
        .or_else(|| meta("article:published_time"))
        .or_else(|| attr(dom, "time[datetime]", "datetime"));
    let updated = schema_value(cx, "dateModified").or_else(|| meta("article:modified_time"));
    (published, updated)
}

/// FR-TRUST-02: (signal, present) for the six names the requirement lists.
pub(crate) fn schema_signals(cx: &RuleCx<'_>) -> [(&'static str, bool); 6] {
    let typed = |t: &str| nodes(cx).any(|n| n.is(t));
    let has = |p: &str| nodes(cx).any(|n| n.has(p));
    [
        ("Article", typed("Article")),
        ("BlogPosting", typed("BlogPosting")),
        // LocalBusiness and its subtypes are Organizations too.
        ("Organization", ORG_TYPES.iter().chain(&crate::schema::LOCAL_TYPES).any(|t| typed(t))),
        ("author", has("author")),
        ("datePublished", has("datePublished")),
        ("dateModified", has("dateModified")),
    ]
}

/// Organization and its common subtypes, besides the LocalBusiness ones.
const ORG_TYPES: [&str; 6] =
    ["Organization", "Corporation", "NGO", "EducationalOrganization", "NewsMediaOrganization", "OnlineBusiness"];

/// Digits only, so "+965 9999-1234" and "(965) 99991234" compare equal.
pub(crate) fn digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

/// Phone numbers on the page: `tel:` links and numbers of 8 to 15 digits in the body text.
pub(crate) fn page_phones(cx: &RuleCx<'_>) -> Vec<String> {
    let mut out: Vec<String> = cx
        .page
        .links
        .iter()
        .filter_map(|l| l.href.trim().strip_prefix("tel:"))
        .map(digits)
        .filter(|d| d.len() >= 7)
        .collect();
    out.extend(PHONE.find_iter(&cx.page.text).map(|m| digits(m.as_str())).filter(|d| (8..=15).contains(&d.len())));
    let mut seen = HashSet::new();
    out.retain(|d| seen.insert(d.clone()));
    out
}

/// Two numbers match when one ends with the other's last 8 digits, so a country code or a
/// leading 0 on one side does not break the match.
pub(crate) fn same_phone(a: &str, b: &str) -> bool {
    let tail = |s: &str| s[s.len().saturating_sub(8)..].to_string();
    !a.is_empty() && !b.is_empty() && tail(a) == tail(b)
}

/// A street address visitors can see: an `<address>` element, PostalAddress microdata or RDFa
/// (both mark up visible text), or a street pattern in the text. JSON-LD alone is not on the page.
pub(crate) fn has_address(cx: &RuleCx<'_>, dom: &Html) -> bool {
    let visible_markup = cx
        .page
        .schema
        .iter()
        .filter(|i| i.source != crate::SchemaSource::JsonLd)
        .flat_map(|i| i.node.walk())
        .any(|n| n.is("PostalAddress") || n.has("streetAddress"));
    !all(dom, "address").is_empty() || visible_markup || STREET.is_match(&cx.page.text)
}

pub(crate) enum Disclosure {
    /// Affiliate links or ads with a disclosure, or a disclosure on its own.
    Disclosed,
    /// Affiliate links or ads, and no disclosure.
    Missing,
    /// Nothing that needs one.
    NotNeeded,
}

pub(crate) fn disclosure(cx: &RuleCx<'_>, html: &str) -> Disclosure {
    let affiliate =
        cx.page.links.iter().any(|l| l.rel.split_whitespace().any(|r| r == "sponsored") || AFFILIATE.is_match(&l.href));
    let ads = ADS.is_match(html);
    match (DISCLOSURE.is_match(&cx.page.text), affiliate || ads) {
        (true, _) => Disclosure::Disclosed,
        (false, true) => Disclosure::Missing,
        (false, false) => Disclosure::NotNeeded,
    }
}

/// True for government, education, military and international bodies' domains, and the
/// bundled list in `data/authoritative-domains.txt`.
pub(crate) fn authoritative(host: &str) -> bool {
    let host = host.trim_start_matches("www.").to_lowercase();
    let parts: Vec<&str> = host.split('.').collect();
    let suffix_ok = match parts.as_slice() {
        [.., tld] if ["gov", "edu", "mil", "int"].contains(tld) => true,
        [.., sld, cc] if cc.len() == 2 && ["gov", "edu", "ac", "gob", "govt", "mil"].contains(sld) => true,
        _ => false,
    };
    suffix_ok || AUTHORITATIVE.iter().any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

/// FR-TRUST-04: (authoritative, total) over distinct outbound links in the main content.
pub(crate) fn citations(cx: &RuleCx<'_>) -> (usize, usize) {
    let mut seen = HashSet::new();
    let hosts: Vec<String> = cx
        .page
        .links
        .iter()
        .filter(|l| l.in_main && !l.internal)
        .filter_map(|l| l.url.as_deref())
        .filter(|u| u.starts_with("http"))
        .filter(|u| seen.insert(u.split('#').next().unwrap_or(u).to_string()))
        .filter_map(|u| url::Url::parse(u).ok()?.host_str().map(str::to_string))
        .collect();
    (hosts.iter().filter(|h| authoritative(h)).count(), hosts.len())
}

/// The first LocalBusiness-type node on the page.
pub(crate) fn local_business<'a>(cx: &'a RuleCx<'_>) -> Option<&'a SchemaNode> {
    nodes(cx).find(|n| crate::schema::LOCAL_TYPES.iter().any(|t| n.is(t)))
}

pub(crate) fn business_profile_link(cx: &RuleCx<'_>) -> Option<String> {
    cx.page.links.iter().filter_map(|l| l.url.as_deref()).find(|u| BUSINESS_PROFILE.is_match(u)).map(str::to_string)
}

/// FR-TRUST-05: which of name, address and phone the schema gives (so could be compared), and
/// those the page text does not show.
pub(crate) fn nap_mismatches(cx: &RuleCx<'_>, business: &SchemaNode) -> (Vec<&'static str>, Vec<String>) {
    let body = cx.page.text.to_lowercase();
    let mut compared = Vec::new();
    let mut out = Vec::new();
    if let Some(phone) = business.values("telephone").find_map(text_of) {
        compared.push("phone");
        let phones = page_phones(cx);
        if !phones.iter().any(|p| same_phone(p, &digits(&phone))) {
            out.push(match phones.first() {
                Some(p) => format!("phone on the page ({p}) differs from the schema ({phone})"),
                None => format!("the schema phone ({phone}) is not on the page"),
            });
        }
    }
    if let Some(name) = business.values("name").find_map(text_of) {
        compared.push("name");
        if !body.contains(&name.to_lowercase()) {
            out.push(format!("the schema name ({name}) is not on the page"));
        }
    }
    let street = business.nodes("address").find_map(|a| a.values("streetAddress").find_map(text_of));
    if let Some(street) = street {
        compared.push("street address");
        if !collapse(&body).contains(&collapse(&street.to_lowercase())) {
            out.push(format!("the schema street address ({street}) is not on the page"));
        }
    }
    (compared, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authoritative_domains() {
        for h in
            ["www.who.int", "data.gov", "cdc.gov", "ox.ac.uk", "en.wikipedia.org", "developer.mozilla.org", "gob.mx"]
        {
            assert!(authoritative(h), "{h}");
        }
        for h in ["myblog.com", "gov.example.com", "wikipedia.org.evil.com"] {
            assert!(!authoritative(h), "{h}");
        }
    }

    #[test]
    fn phones_compare_on_their_last_digits() {
        assert!(same_phone(&digits("+965 9999 1234"), &digits("9999-1234")));
        assert!(same_phone(&digits("(020) 7946 0958"), &digits("+44 20 7946 0958")));
        assert!(!same_phone(&digits("+965 9999 1234"), &digits("+965 9999 4321")));
    }
}
