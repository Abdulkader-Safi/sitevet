//! url-structure rules, ported from the TS package's `src/rules/url-structure.ts`. Thresholds
//! follow Google's URL structure guidance.

use std::sync::Arc;

use url::Url;

use super::kit::js_len;
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "url-structure";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(Lowercase),
        Arc::new(Hyphens),
        Arc::new(NoSpaces),
        Arc::new(Ascii),
        Arc::new(ParamCount),
        Arc::new(Readable),
        Arc::new(Length),
        Arc::new(Depth),
    ]
}

/// JavaScript's `decodeURI`: decodes escapes except those for reserved characters. None when an
/// escape is malformed, where JavaScript throws.
fn decode_uri(s: &str) -> Option<String> {
    const RESERVED: &str = ";/?:@&=+$,#";
    let bytes = s.as_bytes();
    let hex = |i: usize| -> Option<u8> { u8::from_str_radix(s.get(i + 1..i + 3)?, 16).ok() };
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            let c = s[i..].chars().next()?;
            out.push(c);
            i += c.len_utf8();
            continue;
        }
        let lead = hex(i)?;
        if lead < 0x80 {
            let c = char::from(lead);
            if RESERVED.contains(c) {
                out.push_str(&s[i..i + 3])
            } else {
                out.push(c)
            }
            i += 3;
            continue;
        }
        let len = match lead {
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            _ => return None,
        };
        let mut buf = vec![lead];
        for k in 1..len {
            let at = i + 3 * k;
            if bytes.get(at) != Some(&b'%') {
                return None;
            }
            buf.push(hex(at)?);
        }
        out.push_str(std::str::from_utf8(&buf).ok()?);
        i += 3 * len;
    }
    Some(out)
}

/// The parsed URL, its decoded path and its path segments. None when either step fails, as the
/// TS `parts()` helper returns a null URL.
fn parts(raw: &str) -> Option<(Url, String, Vec<String>)> {
    let url = Url::parse(raw).ok()?;
    let path = decode_uri(url.path())?;
    let segments = url.path().split('/').filter(|s| !s.is_empty()).map(str::to_string).collect();
    Some((url, path, segments))
}

fn unparsable() -> Vec<R> {
    vec![R::warn("Could not parse URL")]
}

/// `url.search`: "?" plus the query, or nothing.
fn search(url: &Url) -> String {
    url.query().filter(|q| !q.is_empty()).map(|q| format!("?{q}")).unwrap_or_default()
}

dom_rule!(
    Lowercase,
    "url-structure/lowercase",
    "Lowercase URL",
    Warning,
    "Use lowercase URLs. Mixed case creates duplicate-URL issues because servers often treat cases differently.",
    |cx, dom, html| {
        let Some((url, ..)) = parts(&cx.page.url) else { return unparsable() };
        if url.path().chars().any(|c| c.is_ascii_uppercase()) {
            vec![R::warn("URL path contains uppercase letters").with_evidence(url.path())]
        } else {
            vec![R::pass("URL is lowercase")]
        }
    }
);

dom_rule!(
    Hyphens,
    "url-structure/hyphens",
    "Hyphens not underscores",
    Warning,
    "Separate words with hyphens, not underscores. Google treats hyphens as word separators and underscores as joiners.",
    |cx, dom, html| {
        let path = parts(&cx.page.url).map(|p| p.1).unwrap_or_default();
        if path.contains('_') {
            vec![R::warn("URL uses underscores between words").with_evidence(path)]
        } else {
            vec![R::pass("No underscores in URL")]
        }
    }
);

dom_rule!(
    NoSpaces,
    "url-structure/no-spaces",
    "No spaces in URL",
    Warning,
    "Remove spaces from URLs. Replace them with hyphens. Encoded spaces (%20) are hard to read and share.",
    |cx, dom, html| {
        let before_hash = cx.page.url.split('#').next().unwrap_or("");
        if before_hash.contains("%20") || before_hash.chars().any(char::is_whitespace) {
            vec![R::warn("URL contains spaces or %20").with_evidence(cx.page.url.clone())]
        } else {
            vec![R::pass("No spaces in URL")]
        }
    }
);

dom_rule!(
    Ascii,
    "url-structure/ascii",
    "ASCII or encoded characters",
    Info,
    "Percent-encode non-ASCII characters in URLs so every client handles them the same way.",
    |cx, dom, html| {
        let Some((_, raw, _)) = parts(&cx.page.url) else { return unparsable() };
        if raw.is_ascii() {
            vec![R::pass("URL uses ASCII or encoded characters")]
        } else {
            vec![R::warn("URL contains raw non-ASCII characters").with_evidence(raw)]
        }
    }
);

dom_rule!(
    ParamCount,
    "url-structure/param-count",
    "Few query parameters",
    Info,
    "Minimise query parameters. Many parameters multiply crawlable URLs and dilute ranking signals. Aim for two or fewer.",
    |cx, dom, html| {
        let Some((url, ..)) = parts(&cx.page.url) else { return unparsable() };
        let n = url.query_pairs().count();
        if n <= 2 {
            vec![R::pass(format!("{n} query parameter(s)"))]
        } else {
            vec![R::warn(format!("{n} query parameters")).with_evidence(search(&url))]
        }
    }
);

dom_rule!(
    Readable,
    "url-structure/readable",
    "Readable, word-based path",
    Info,
    "Use descriptive words in URLs instead of bare IDs. /products/leather-wallet beats /p?id=42.",
    |cx, dom, html| {
        let Some((url, _, segments)) = parts(&cx.page.url) else { return unparsable() };
        let last = segments.last().map(String::as_str).unwrap_or("");
        let has = |k: &str| url.query_pairs().any(|(key, _)| key == k);
        let id_like = (!last.is_empty() && last.bytes().all(|b| b.is_ascii_digit())) || has("id") || has("p");
        if segments.is_empty() {
            return vec![R::pass("Home URL")];
        }
        if id_like {
            vec![R::warn("URL relies on a numeric ID rather than words").with_evidence(format!(
                "{}{}",
                url.path(),
                search(&url)
            ))]
        } else {
            vec![R::pass("URL uses readable words")]
        }
    }
);

dom_rule!(
    Length,
    "url-structure/length",
    "URL length",
    Info,
    "Keep URLs short. Long URLs are harder to share and can be truncated in search results. Aim under 100 characters.",
    |cx, dom, html| {
        let len = js_len(&cx.page.url);
        if len <= 100 {
            return vec![R::pass(format!("URL is {len} characters"))];
        }
        if len <= 115 {
            return vec![R::warn(format!("Long URL ({len} characters)"))];
        }
        vec![R::fail(format!("Very long URL ({len} characters)")).with_evidence(cx.page.url.clone())]
    }
);

dom_rule!(
    Depth,
    "url-structure/depth",
    "Shallow path depth",
    Info,
    "Avoid deep nesting. Keep important pages within a few clicks of the root. Aim for four path segments or fewer.",
    |cx, dom, html| {
        let segments = parts(&cx.page.url).map(|p| p.2).unwrap_or_default();
        if segments.len() <= 4 {
            return vec![R::pass(format!("{} path segment(s)", segments.len()))];
        }
        vec![
            R::warn(format!("Deep path ({} segments)", segments.len()))
                .with_evidence(format!("/{}", segments.join("/"))),
        ]
    }
);

#[cfg(test)]
mod tests {
    use super::decode_uri;
    use crate::Status::{Fail, Pass, Warn};
    use crate::rules::testkit::page;

    fn at(url: &str) -> crate::rules::testkit::Page {
        page("<p>x</p>").url(url)
    }

    #[test]
    fn decode_uri_matches_javascript() {
        assert_eq!(decode_uri("/caf%C3%A9/a%20b%2Fc").as_deref(), Some("/café/a b%2Fc"));
        assert_eq!(decode_uri("/bad%E9"), None);
        assert_eq!(decode_uri("/x%ZZ"), None);
    }

    #[test]
    fn case_underscores_spaces_ascii() {
        let clean = at("https://example.com/blog/clean-url");
        for id in ["lowercase", "hyphens", "no-spaces", "ascii", "param-count", "readable", "length", "depth"] {
            assert_eq!(clean.status(&format!("url-structure/{id}")), Pass, "{id}");
        }
        assert_eq!(at("https://example.com/Blog").status("url-structure/lowercase"), Warn);
        assert_eq!(at("https://example.com/my_page").status("url-structure/hyphens"), Warn);
        assert_eq!(at("https://example.com/a%20b").status("url-structure/no-spaces"), Warn);
        let accented = at("https://example.com/caf%C3%A9");
        assert_eq!(accented.status("url-structure/ascii"), Warn);
        assert_eq!(accented.results("url-structure/ascii")[0].evidence.as_deref(), Some("/café"));
        assert_eq!(at("https://example.com/bad%E9").status("url-structure/ascii"), Warn);
        assert_eq!(at("https://example.com/bad%E9").message("url-structure/ascii"), "Could not parse URL");
    }

    #[test]
    fn params_ids_length_depth() {
        assert_eq!(at("https://example.com/s?a=1&b=2&c=3").message("url-structure/param-count"), "3 query parameters");
        assert_eq!(at("https://example.com/products/42").status("url-structure/readable"), Warn);
        assert_eq!(at("https://example.com/p?id=42").status("url-structure/readable"), Warn);
        assert_eq!(at("https://example.com/").message("url-structure/readable"), "Home URL");
        let long = |n: usize| at(&format!("https://example.com/{}", "a".repeat(n - 20)));
        assert_eq!(long(110).status("url-structure/length"), Warn);
        assert_eq!(long(130).status("url-structure/length"), Fail);
        assert_eq!(at("https://example.com/a/b/c/d/e").message("url-structure/depth"), "Deep path (5 segments)");
    }
}
