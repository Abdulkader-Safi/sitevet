//! Page basics as values, from the TS package's `src/metrics/page.ts`.

use std::sync::Arc;

use scraper::Html;
use serde_json::Value;

use super::{in_range, num, txt, up_to, value};
use crate::report::ValueStatus;
use crate::rules::kit::{all, attr, count, first_text_collapsed, header, js_len, re, text};
use crate::{Collector, MetricInput};

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(CorePageBasics), Arc::new(ContentPageBasics), Arc::new(Response), Arc::new(Document), Arc::new(Dates)]
}

/// Inline `display:none` and the `hidden` attribute. No cascade, so no CSS files.
fn hidden_count(dom: &Html) -> usize {
    let none = re(r"(?i)display\s*:\s*none");
    count(dom, "[hidden]")
        + all(dom, "[style]").iter().filter(|e| none.is_match(e.value().attr("style").unwrap_or(""))).count()
}

/// The first string value for `key` anywhere in the page's JSON-LD.
fn from_json_ld(dom: &Html, key: &str) -> Option<String> {
    fn walk(node: &Value, key: &str) -> Option<String> {
        match node {
            Value::Array(items) => items.iter().find_map(|i| walk(i, key)),
            Value::Object(obj) => match obj.get(key) {
                Some(Value::String(s)) => Some(s.clone()),
                _ => obj.values().find_map(|v| walk(v, key)),
            },
            _ => None,
        }
    }
    all(dom, crate::rules::JSON_LD)
        .into_iter()
        .find_map(|el| serde_json::from_str::<Value>(&text(el)).ok().and_then(|v| walk(&v, key)))
}

fn meta_date(dom: &Html, names: &[&str]) -> Option<String> {
    names.iter().find_map(|n| {
        let v =
            [format!(r#"meta[property="{n}"]"#), format!(r#"meta[name="{n}"]"#), format!(r#"meta[itemprop="{n}"]"#)]
                .iter()
                .find_map(|css| attr(dom, css, "content"))?;
        let v = v.trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// A date is only useful if it parses. Accepts what `Date.parse` reliably reads: ISO 8601 and
/// the RFC 2822 style of HTTP headers.
// ponytail: V8's Date.parse also reads looser forms ("January 15, 2026"); add them if real pages need it.
fn date_status(raw: Option<&str>) -> ValueStatus {
    let Some(raw) = raw else { return ValueStatus::Missing };
    let iso = re(r"^\d{4}(-\d{2}(-\d{2}([T ]\d{2}:\d{2}(:\d{2}(\.\d+)?)?(Z|[+-]\d{2}:?\d{2})?)?)?)?$");
    let rfc = re(
        r"(?i)^([a-z]{3},\s*)?\d{1,2}\s+(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\s+\d{4}(\s+\d{2}:\d{2}(:\d{2})?)?(\s+([a-z]{2,4}|[+-]\d{4}))?$",
    );
    if iso.is_match(raw.trim()) || rfc.is_match(raw.trim()) { ValueStatus::Ok } else { ValueStatus::Warn }
}

dom_collector!(CorePageBasics, "core-seo", "core-seo/page-basics", |cx, dom, html| {
    let title = first_text_collapsed(dom, "title");
    let desc = attr(dom, r#"meta[name="description"]"#, "content").unwrap_or_default().trim().to_string();
    let h1s = count(dom, "h1");
    let h1 = first_text_collapsed(dom, "h1");
    let len = |s: &str| (!s.is_empty()).then(|| js_len(s));
    let chars = |id: &str, label: &str, n: Option<usize>, min: usize, max: usize| {
        MetricInput::value(id, label, n.and_then(|n| num(n as f64)), Some("chars"))
            .with_status(n.map_or(ValueStatus::Missing, |n| in_range(n, min, max)))
            .with_note(format!("Recommended {min} to {max}"))
    };
    vec![
        chars("core-seo/title-chars", "Title length", len(&title), 30, 60),
        chars("core-seo/description-chars", "Description length", len(&desc), 70, 160),
        value("core-seo/h1-count", "H1 count", num(h1s as f64))
            .with_status(match h1s {
                1 => ValueStatus::Ok,
                0 => ValueStatus::Missing,
                _ => ValueStatus::Warn,
            })
            .with_note("Exactly one"),
        chars("core-seo/h1-chars", "H1 length", len(&h1), 10, 70),
    ]
});

dom_collector!(ContentPageBasics, "content", "content/page-basics", |cx, dom, html| {
    let lang = attr(dom, "html", "lang").map(|l| l.trim().to_string()).filter(|l| !l.is_empty());
    let status = if lang.is_some() { ValueStatus::Ok } else { ValueStatus::Missing };
    vec![
        value("content/declared-language", "Declared language", lang.and_then(txt))
            .with_status(status)
            .with_note("The lang attribute on <html>"),
        value("content/hidden-elements", "Hidden elements", num(hidden_count(dom) as f64))
            .with_note("Inline display:none or the hidden attribute"),
    ]
});

dom_collector!(Response, "crawlability", "crawlability/response", |cx, dom, html| {
    let status = cx.page.status;
    let ms = cx.page.response_ms as f64;
    vec![
        value("crawlability/http-status", "HTTP status", num(status)).with_status(match status {
            200..=299 => ValueStatus::Ok,
            0..=399 => ValueStatus::Warn,
            _ => ValueStatus::Bad,
        }),
        MetricInput::value("crawlability/response-time", "Response time", num(ms), Some("ms"))
            .with_status(up_to(ms, 800.0, 1800.0)),
    ]
});

dom_collector!(Document, "performance", "performance/document", |cx, dom, html| {
    let kb = (js_len(html) as f64 / 1000.0).round();
    vec![
        MetricInput::value("performance/html-size", "HTML size", num(kb), Some("KB"))
            .with_status(up_to(kb, 100.0, 200.0))
            .with_note("Raw document, before assets"),
    ]
});

dom_collector!(Dates, "eeat", "eeat/dates", |cx, dom, html| {
    let published = meta_date(dom, &["article:published_time", "datePublished", "date", "pubdate"])
        .or_else(|| from_json_ld(dom, "datePublished"));
    let updated = meta_date(dom, &["article:modified_time", "dateModified", "last-modified"])
        .or_else(|| from_json_ld(dom, "dateModified"))
        .or_else(|| header(cx.page, "last-modified"));
    vec![
        value("eeat/published-date", "Published", published.clone().and_then(txt))
            .with_status(date_status(published.as_deref())),
        value("eeat/updated-date", "Updated", updated.clone().and_then(txt))
            .with_status(date_status(updated.as_deref())),
    ]
});

#[cfg(test)]
mod tests {
    use super::date_status;
    use crate::report::ValueStatus;

    #[test]
    fn dates_parse_like_javascript() {
        for ok in ["2026-01-15T09:00:00Z", "2026-03-02", "2026-03-02T10:00:00+02:00", "Tue, 10 Nov 2026 09:30:00 GMT"] {
            assert_eq!(date_status(Some(ok)), ValueStatus::Ok, "{ok}");
        }
        assert_eq!(date_status(Some("last week")), ValueStatus::Warn);
        assert_eq!(date_status(None), ValueStatus::Missing);
    }
}
