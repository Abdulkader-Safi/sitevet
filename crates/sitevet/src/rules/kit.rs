//! Helpers every built-in rule and collector uses. They read the DOM the way the TS package's
//! cheerio calls do, so a port reads line for line like its source.

use scraper::{ElementRef, Html, Selector};

use crate::PageData;

/// Defines a rule as a unit struct. The file's `CATEGORY` constant names its category.
///
/// `dom_rule!` rules run in [`crate::Stage::Dom`] and get `dom: &Html` and `html: &str`.
macro_rules! dom_rule {
    ($name:ident, $id:literal, $title:literal, $severity:ident, $fix:literal, |$cx:ident, $dom:ident, $html:ident| $body:block) => {
        pub(crate) struct $name;

        impl $crate::Rule for $name {
            fn meta(&self) -> $crate::RuleMeta {
                let mut m =
                    $crate::RuleMeta::new($id, CATEGORY, $title, $crate::Severity::$severity, $crate::Stage::Dom);
                m.fix = Some($fix);
                m
            }

            #[allow(unused_variables)]
            fn run(&self, $cx: &$crate::RuleCx<'_>) -> Vec<$crate::RuleResult> {
                let Some($dom) = $cx.dom else { return Vec::new() };
                let $html = $cx.html.unwrap_or("");
                $body
            }
        }
    };
}

/// A rule for a later stage: [`crate::Stage::Enriched`] or [`crate::Stage::Rendered`].
macro_rules! rule {
    ($name:ident, $id:literal, $title:literal, $severity:ident, $stage:ident, $fix:literal, |$cx:ident| $body:block) => {
        pub(crate) struct $name;

        impl $crate::Rule for $name {
            fn meta(&self) -> $crate::RuleMeta {
                let mut m =
                    $crate::RuleMeta::new($id, CATEGORY, $title, $crate::Severity::$severity, $crate::Stage::$stage);
                m.fix = Some($fix);
                m
            }

            fn run(&self, $cx: &$crate::RuleCx<'_>) -> Vec<$crate::RuleResult> {
                $body
            }
        }
    };
}

/// Defines a DOM-stage collector as a unit struct.
macro_rules! dom_collector {
    ($name:ident, $category:literal, $id:literal, |$cx:ident, $dom:ident, $html:ident| $body:block) => {
        pub(crate) struct $name;

        impl $crate::Collector for $name {
            fn meta(&self) -> $crate::CollectorMeta {
                $crate::CollectorMeta::new($id, $category, $crate::Stage::Dom)
            }

            #[allow(unused_variables)]
            fn collect(&self, $cx: &$crate::RuleCx<'_>) -> Vec<$crate::MetricInput> {
                let Some($dom) = $cx.dom else { return Vec::new() };
                let $html = $cx.html.unwrap_or("");
                $body
            }
        }
    };
}

// ponytail: selectors are parsed on every call, a few microseconds each. Cache them if rule
// CPU time per page ever nears NFR-01's 50 ms.
#[allow(clippy::expect_used)] // every selector is a literal in this crate, covered by its rule's tests
pub(crate) fn sel(css: &str) -> Selector {
    Selector::parse(css).expect("valid selector")
}

pub(crate) fn all<'a>(dom: &'a Html, css: &str) -> Vec<ElementRef<'a>> {
    dom.select(&sel(css)).collect()
}

pub(crate) fn first<'a>(dom: &'a Html, css: &str) -> Option<ElementRef<'a>> {
    dom.select(&sel(css)).next()
}

pub(crate) fn count(dom: &Html, css: &str) -> usize {
    dom.select(&sel(css)).count()
}

/// cheerio `.text()`: every descendant text node, scripts included.
pub(crate) fn text(e: ElementRef) -> String {
    e.text().collect()
}

/// `$(css).first().text().trim()`
pub(crate) fn first_text(dom: &Html, css: &str) -> String {
    first(dom, css).map(|e| text(e).trim().to_string()).unwrap_or_default()
}

/// `$(css).first().text().replace(/\s+/g, " ").trim()`
pub(crate) fn first_text_collapsed(dom: &Html, css: &str) -> String {
    first(dom, css).map(|e| collapse(&text(e))).unwrap_or_default()
}

pub(crate) fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `$(css).attr(name)`: the attribute of the first match.
pub(crate) fn attr(dom: &Html, css: &str, name: &str) -> Option<String> {
    first(dom, css).and_then(|e| e.value().attr(name)).map(str::to_string)
}

/// `$(css).attr(name)?.trim()`, with an empty result as None.
pub(crate) fn attr_trim(dom: &Html, css: &str, name: &str) -> Option<String> {
    attr(dom, css, name).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// A response header by lowercase name. Repeated headers are joined with ", ".
pub(crate) fn header(page: &PageData, name: &str) -> Option<String> {
    let values: Vec<&str> = page.headers.iter().filter(|(k, _)| k == name).map(|(_, v)| v.as_str()).collect();
    (!values.is_empty()).then(|| values.join(", "))
}

/// JavaScript string length (UTF-16 units), which the TS thresholds count in.
pub(crate) fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Trims a long string for a table cell. Whitespace collapsed, cut with "...".
pub(crate) fn clip(s: &str, max: usize) -> String {
    let t = collapse(s);
    if t.chars().count() > max { format!("{}...", t.chars().take(max - 1).collect::<String>()) } else { t }
}

/// A case-insensitive regex, compiled per call like the selectors.
#[allow(clippy::expect_used)] // literal patterns, covered by tests
pub(crate) fn re(pattern: &str) -> regex::Regex {
    regex::Regex::new(pattern).expect("valid regex")
}
