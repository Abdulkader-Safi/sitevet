//! performance rules, ported from the TS package's `src/rules/performance.ts`. The Core Web
//! Vitals rules run in the rendered stage (M4); the rest read the response.

use std::sync::Arc;

use super::kit::{all, count, header, js_len, re, text};
use super::{PerfMetrics, RenderedData};
use crate::{Rule, RuleResult as R};

const CATEGORY: &str = "performance";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(RenderBlockingScripts),
        Arc::new(StylesheetCount),
        Arc::new(InlineScriptSize),
        Arc::new(TextCompression),
        Arc::new(CacheControl),
        Arc::new(HtmlWeight),
        Arc::new(Lcp),
        Arc::new(Cls),
        Arc::new(Ttfb),
        Arc::new(DomSize),
        Arc::new(PageWeight),
        Arc::new(RequestCount),
        Arc::new(FontDisplay),
        Arc::new(HttpVersion),
        Arc::new(ResourceHints),
        Arc::new(ThirdPartyScripts),
    ]
}

/// The site a host belongs to: its last two labels, so `cdn.example.com` is `example.com`.
// ponytail: two labels, so `shop.example.co.uk` and `blog.other.co.uk` look like one site. A
// public suffix list fixes that if it ever matters.
fn site_of(host: &str) -> String {
    let labels: Vec<&str> = host.trim_start_matches("www.").split('.').collect();
    labels[labels.len().saturating_sub(2)..].join(".")
}

/// The origin of an absolute URL on another site than the page, or None.
pub(crate) fn third_party_origin(url: &str, page_url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok().filter(|u| matches!(u.scheme(), "http" | "https"))?;
    let page_host = url::Url::parse(page_url).ok()?.host_str()?.to_string();
    (site_of(u.host_str()?) != site_of(&page_host)).then(|| u.origin().ascii_serialization())
}

static FONT_FACE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| re(r"(?is)@font-face\s*\{[^}]*\}"));
static FONT_FILE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| re(r#"(?i)url\(\s*['"]?([^'")]+\.(?:woff2?|ttf|otf|eot))"#));
const GOOGLE_FONTS: &str = r#"link[href*="fonts.googleapis.com/css"]"#;

/// (web fonts declared, ones without font-display) in inline `@font-face` rules and Google Fonts links.
pub(crate) fn font_display(dom: &scraper::Html) -> (usize, usize) {
    let css: String = all(dom, "style").into_iter().map(text).collect();
    let faces: Vec<&str> = FONT_FACE.find_iter(&css).map(|m| m.as_str()).collect();
    let google: Vec<String> =
        all(dom, GOOGLE_FONTS).into_iter().filter_map(|l| l.value().attr("href").map(str::to_string)).collect();
    let missing = faces.iter().filter(|f| !f.contains("font-display")).count()
        + google.iter().filter(|h| !h.contains("display=")).count();
    (faces.len() + google.len(), missing)
}

/// (font files referenced in inline CSS, font files preloaded).
pub(crate) fn font_files(dom: &scraper::Html) -> (usize, usize) {
    let css: String = all(dom, "style").into_iter().map(text).collect();
    let mut files: Vec<String> = FONT_FILE.captures_iter(&css).map(|c| c[1].to_string()).collect();
    files.sort();
    files.dedup();
    (files.len(), count(dom, r#"link[rel~="preload"][as="font"]"#))
}

dom_rule!(
    FontDisplay,
    "performance/font-display",
    "Font loading",
    Info,
    "Set font-display: swap (or optional) on every @font-face, and add &display=swap to Google Fonts links, so text shows while fonts load.",
    |cx, dom, html| {
        match font_display(dom) {
            (0, _) => vec![R::info("No web fonts declared in the HTML (external stylesheets are not read)")],
            (n, 0) => vec![R::pass(format!("font-display set on {n} web font declaration(s)"))],
            (n, missing) => vec![R::warn(format!("{missing} of {n} web font declaration(s) without font-display"))],
        }
    }
);

dom_rule!(
    HttpVersion,
    "performance/http-version",
    "HTTP/2 or HTTP/3",
    Warning,
    "Serve the site over HTTP/2 or HTTP/3. Most hosts and CDNs turn it on with one setting.",
    |cx, dom, html| {
        let h3 = header(cx.page, "alt-svc").is_some_and(|v| v.contains("h3"));
        match cx.page.http_version.as_deref() {
            None => vec![R::info("No response to read the HTTP version from")],
            Some(v @ ("HTTP/2" | "HTTP/3")) => vec![R::pass(format!("Served over {v}"))],
            Some(v) if h3 => vec![R::pass(format!("Served over {v}, and HTTP/3 is offered"))],
            Some(v) => vec![R::warn(format!("Served over {v}"))],
        }
    }
);

dom_rule!(
    ResourceHints,
    "performance/resource-hints",
    "Resource hints for third-party origins",
    Info,
    "Add <link rel=\"preconnect\"> (or dns-prefetch) for each third-party origin the page loads scripts, styles or fonts from.",
    |cx, dom, html| {
        let page = &cx.page.final_url;
        let mut needed: Vec<String> = cx
            .page
            .scripts
            .iter()
            .filter_map(|s| s.src.as_deref())
            .chain(cx.page.stylesheets.iter().map(String::as_str))
            .filter_map(|u| third_party_origin(u, page))
            .collect();
        if count(dom, GOOGLE_FONTS) > 0 {
            needed.push("https://fonts.gstatic.com".into());
        }
        needed.sort();
        needed.dedup();
        let hinted: Vec<String> = all(dom, r#"link[rel~="preconnect"][href], link[rel~="dns-prefetch"][href]"#)
            .into_iter()
            .filter_map(|l| l.value().attr("href"))
            .filter_map(|h| url::Url::parse(page).ok()?.join(h).ok())
            .map(|u| u.origin().ascii_serialization())
            .collect();
        let missing: Vec<&String> = needed.iter().filter(|o| !hinted.contains(o)).collect();
        if needed.is_empty() {
            vec![R::info("No third-party origins for scripts, styles or fonts")]
        } else if missing.is_empty() {
            vec![R::pass(format!("All {} third-party origin(s) have a preconnect or dns-prefetch", needed.len()))]
        } else {
            let list: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
            vec![
                R::warn(format!("{} third-party origin(s) without preconnect or dns-prefetch", missing.len()))
                    .with_evidence(list.join(", ")),
            ]
        }
    }
);

/// More third-party scripts than this, or more bytes, warns.
const THIRD_PARTY_MAX: usize = 10;
const THIRD_PARTY_MAX_BYTES: u64 = 500 * 1024;

/// Third-party script URLs on the page, with their size when the server sent one.
pub(crate) fn third_party_scripts(cx: &crate::RuleCx<'_>) -> Vec<(String, Option<u64>)> {
    let mut out: Vec<(String, Option<u64>)> = cx
        .page
        .scripts
        .iter()
        .filter_map(|s| s.src.clone())
        .filter(|u| third_party_origin(u, &cx.page.final_url).is_some())
        .map(|u| {
            let bytes = cx.links.get(&u).and_then(|l| l.bytes);
            (u, bytes)
        })
        .collect();
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

rule!(
    ThirdPartyScripts,
    "performance/third-party-scripts",
    "Third-party scripts",
    Info,
    Enriched,
    "Drop third-party scripts you do not need, and load the rest with async or defer.",
    |cx| {
        let scripts = third_party_scripts(cx);
        if scripts.is_empty() {
            return vec![R::info("No third-party scripts")];
        }
        let known: u64 = scripts.iter().filter_map(|(_, b)| *b).sum();
        let unknown = scripts.iter().filter(|(_, b)| b.is_none()).count();
        let size = match (unknown, known) {
            (0, _) => format!("{} KB", known.div_ceil(1024)),
            (_, 0) => "size unknown (the server sent no Content-Length)".to_string(),
            _ => format!("{} KB plus {unknown} of unknown size", known.div_ceil(1024)),
        };
        let msg = format!("{} third-party script(s), {size}", scripts.len());
        if scripts.len() > THIRD_PARTY_MAX || known > THIRD_PARTY_MAX_BYTES {
            vec![R::warn(msg)]
        } else {
            vec![R::pass(msg)]
        }
    }
);

dom_rule!(
    RenderBlockingScripts,
    "performance/render-blocking-scripts",
    "Render-blocking scripts",
    Warning,
    "Add defer or async to scripts in <head>, or move them before </body>, so they do not block rendering.",
    |cx, dom, html| {
        let blocking = all(dom, "head script[src]")
            .into_iter()
            .filter(|s| {
                let e = s.value();
                e.attr("defer").is_none() && e.attr("async").is_none() && e.attr("type").unwrap_or("") != "module"
            })
            .count();
        if blocking == 0 {
            vec![R::pass("No render-blocking scripts in head")]
        } else {
            vec![R::warn(format!("{blocking} render-blocking script(s) in head"))]
        }
    }
);

dom_rule!(
    StylesheetCount,
    "performance/stylesheet-count",
    "Stylesheet count",
    Info,
    "Combine stylesheets. Many separate CSS files add round trips. Aim for a handful.",
    |cx, dom, html| {
        let n = count(dom, r#"link[rel="stylesheet"]"#);
        if n <= 5 { vec![R::pass(format!("{n} stylesheet(s)"))] } else { vec![R::warn(format!("{n} stylesheets"))] }
    }
);

dom_rule!(
    InlineScriptSize,
    "performance/inline-script-size",
    "Inline script size",
    Info,
    "Move large inline scripts to cacheable external files.",
    |cx, dom, html| {
        let bytes: usize = all(dom, "script:not([src])").into_iter().map(|s| js_len(&text(s))).sum();
        let kb = bytes as f64 / 1000.0;
        if bytes <= 30_000 {
            vec![R::pass(format!("{kb:.1}KB inline JS"))]
        } else {
            vec![R::warn(format!("{kb:.1}KB of inline JavaScript"))]
        }
    }
);

dom_rule!(
    TextCompression,
    "performance/text-compression",
    "Text compression",
    Warning,
    "Enable gzip or Brotli compression on HTML, CSS, and JS responses to cut transfer size.",
    |cx, dom, html| {
        let enc = header(cx.page, "content-encoding").unwrap_or_default().to_lowercase();
        if re("gzip|br|deflate|zstd").is_match(&enc) {
            vec![R::pass(format!("Compressed ({enc})"))]
        } else {
            vec![R::warn("HTML response is not compressed")]
        }
    }
);

dom_rule!(
    CacheControl,
    "performance/cache-control",
    "Cache headers",
    Info,
    "Send a Cache-Control header so browsers and CDNs can cache responses.",
    |cx, dom, html| {
        match header(cx.page, "cache-control").filter(|v| !v.is_empty()) {
            Some(v) => vec![R::pass(format!("Cache-Control: {v}"))],
            None => vec![R::warn("No Cache-Control header")],
        }
    }
);

dom_rule!(
    HtmlWeight,
    "performance/html-weight",
    "HTML document size",
    Info,
    "Reduce the raw HTML size. Large documents are slow to parse. Aim under 100KB.",
    |cx, dom, html| {
        let kb = js_len(html) as f64 / 1000.0;
        if kb <= 100.0 {
            return vec![R::pass(format!("{kb:.0}KB HTML"))];
        }
        if kb <= 200.0 {
            return vec![R::warn(format!("Large HTML ({kb:.0}KB)"))];
        }
        vec![R::fail(format!("Very large HTML ({kb:.0}KB)"))]
    }
);

/// Unrendered pages warn, pages without numbers get a neutral info, the rest go to `judge`.
fn with_metrics(rendered: Option<&RenderedData>, judge: impl Fn(&PerfMetrics) -> R) -> Vec<R> {
    let Some(data) = rendered.filter(|d| d.ok) else {
        return vec![R::warn("Page could not be rendered for performance checks")];
    };
    match &data.metrics {
        Some(m) => vec![judge(m)],
        None => vec![R::info("Not measured (no performance provider)")],
    }
}

rule!(
    Lcp,
    "performance/lcp",
    "Largest Contentful Paint",
    Error,
    Rendered,
    "Optimise the largest above-the-fold element. Compress hero images, preload key assets, and cut render-blocking resources to get LCP under 2.5s.",
    |cx| {
        with_metrics(cx.rendered, |m| {
            if m.lcp_ms == 0.0 {
                return R::info("LCP not measured");
            }
            let s = m.lcp_ms / 1000.0;
            pick(
                m.lcp_ms,
                2500.0,
                4000.0,
                format!("LCP {s:.2}s"),
                format!("LCP needs work ({s:.2}s)"),
                format!("Poor LCP ({s:.2}s)"),
            )
        })
    }
);

/// pass up to `good`, warn up to `poor`, fail past it.
fn pick(n: f64, good: f64, poor: f64, pass: String, warn: String, fail: String) -> R {
    if n <= good {
        R::pass(pass)
    } else if n <= poor {
        R::warn(warn)
    } else {
        R::fail(fail)
    }
}

rule!(
    Cls,
    "performance/cls",
    "Cumulative Layout Shift",
    Warning,
    Rendered,
    "Set width and height on images and embeds, and reserve space for dynamic content to keep CLS under 0.1.",
    |cx| {
        with_metrics(cx.rendered, |m| {
            let c = m.cls;
            pick(c, 0.1, 0.25, format!("CLS {c}"), format!("CLS needs work ({c})"), format!("Poor CLS ({c})"))
        })
    }
);

rule!(
    Ttfb,
    "performance/ttfb",
    "Time to First Byte",
    Warning,
    Rendered,
    "Reduce server response time with caching, a CDN, or faster backend rendering to get TTFB under 800ms.",
    |cx| {
        with_metrics(cx.rendered, |m| {
            let t = m.ttfb_ms;
            pick(
                t,
                800.0,
                1800.0,
                format!("TTFB {t}ms"),
                format!("Slow TTFB ({t}ms)"),
                format!("Very slow TTFB ({t}ms)"),
            )
        })
    }
);

rule!(
    DomSize,
    "performance/dom-size",
    "DOM size",
    Info,
    Rendered,
    "Reduce the number of DOM nodes. Large DOMs slow rendering and increase memory use. Aim for under 1500 elements.",
    |cx| {
        with_metrics(cx.rendered, |m| {
            let n = m.dom_nodes;
            pick(
                f64::from(n),
                1500.0,
                3000.0,
                format!("{n} DOM nodes"),
                format!("Large DOM ({n} nodes)"),
                format!("Excessive DOM ({n} nodes)"),
            )
        })
    }
);

rule!(
    PageWeight,
    "performance/page-weight",
    "Page weight",
    Warning,
    Rendered,
    "Cut total transfer size. Compress images, minify assets, and remove unused code. Aim to stay under 3MB.",
    |cx| {
        with_metrics(cx.rendered, |m| {
            let mb = m.transfer_bytes as f64 / 1_000_000.0;
            pick(
                mb,
                3.0,
                6.0,
                format!("{mb:.2}MB transferred"),
                format!("Heavy page ({mb:.2}MB)"),
                format!("Very heavy page ({mb:.2}MB)"),
            )
        })
    }
);

rule!(
    RequestCount,
    "performance/request-count",
    "Request count",
    Info,
    Rendered,
    "Reduce the number of requests by bundling assets, inlining small resources, and lazy-loading non-critical content.",
    |cx| {
        with_metrics(cx.rendered, |m| {
            let n = m.requests;
            pick(
                f64::from(n),
                80.0,
                150.0,
                format!("{n} requests"),
                format!("Many requests ({n})"),
                format!("Excessive requests ({n})"),
            )
        })
    }
);

#[cfg(test)]
mod tests {
    use crate::Status::{self, Fail, Info, Pass, Warn};
    use crate::rules::testkit::page;
    use crate::rules::{PerfMetrics, RenderedData};

    #[test]
    fn static_checks() {
        let p = page(
            r#"<head><script src="/a.js"></script><script src="/b.js" defer></script><script type="module" src="/m.js"></script></head>"#,
        );
        assert_eq!(p.message("performance/render-blocking-scripts"), "1 render-blocking script(s) in head");
        assert_eq!(
            page(r#"<head><script src="/a.js" async></script></head>"#).status("performance/render-blocking-scripts"),
            Pass
        );
        let css: String = (0..6).map(|i| format!(r#"<link rel="stylesheet" href="/{i}.css">"#)).collect();
        assert_eq!(page(&css).message("performance/stylesheet-count"), "6 stylesheets");
        assert_eq!(page("<script>var a=1;</script>").message("performance/inline-script-size"), "0.0KB inline JS");
        assert_eq!(
            page(&format!("<script>{}</script>", "x".repeat(30_001))).status("performance/inline-script-size"),
            Warn
        );
        assert_eq!(
            page("<p>x</p>").header("content-encoding", "br").message("performance/text-compression"),
            "Compressed (br)"
        );
        assert_eq!(page("<p>x</p>").status("performance/text-compression"), Warn);
        assert_eq!(
            page("<p>x</p>").header("cache-control", "max-age=60").message("performance/cache-control"),
            "Cache-Control: max-age=60"
        );
        assert_eq!(page("<p>x</p>").status("performance/cache-control"), Warn);
        assert_eq!(page("<p>x</p>").status("performance/html-weight"), Pass);
        assert_eq!(page(&"x".repeat(150_000)).message("performance/html-weight"), "Large HTML (150KB)");
        assert_eq!(page(&"x".repeat(250_000)).status("performance/html-weight"), Fail);
    }

    fn rendered(m: PerfMetrics) -> crate::rules::testkit::Page {
        let mut p = page("<p>x</p>");
        p.rendered = Some(RenderedData { ok: true, metrics: Some(m), ..Default::default() });
        p
    }

    #[test]
    fn web_vitals() {
        let good = rendered(PerfMetrics {
            ttfb_ms: 200.0,
            lcp_ms: 1200.0,
            cls: 0.05,
            dom_nodes: 900,
            requests: 40,
            transfer_bytes: 1_500_000,
            ..Default::default()
        });
        for id in ["lcp", "cls", "ttfb", "dom-size", "page-weight", "request-count"] {
            assert_eq!(good.status(&format!("performance/{id}")), Pass, "{id}");
        }
        assert_eq!(good.message("performance/lcp"), "LCP 1.20s");
        assert_eq!(good.message("performance/cls"), "CLS 0.05");
        let poor = rendered(PerfMetrics {
            ttfb_ms: 2000.0,
            lcp_ms: 5000.0,
            cls: 0.3,
            dom_nodes: 4000,
            requests: 200,
            transfer_bytes: 7_000_000,
            ..Default::default()
        });
        for id in ["lcp", "cls", "ttfb", "dom-size", "page-weight", "request-count"] {
            assert_eq!(poor.status(&format!("performance/{id}")), Fail, "{id}");
        }
        let mid = rendered(PerfMetrics { lcp_ms: 3000.0, cls: 0.2, ..Default::default() });
        assert_eq!(mid.message("performance/lcp"), "LCP needs work (3.00s)");
        assert_eq!(mid.status("performance/cls"), Warn);
        assert_eq!(rendered(PerfMetrics::default()).status("performance/lcp"), Info);
        assert_eq!(page("<p>x</p>").status("performance/lcp"), Warn);
        let mut unmeasured = page("<p>x</p>");
        unmeasured.rendered = Some(RenderedData { ok: true, ..Default::default() });
        assert_eq!(unmeasured.message("performance/ttfb"), "Not measured (no performance provider)");
    }

    #[test]
    fn font_display() {
        let none = page("<p>x</p>");
        assert_eq!(
            none.message("performance/font-display"),
            "No web fonts declared in the HTML (external stylesheets are not read)"
        );
        let good = page(
            r#"<style>@font-face { font-family: A; src: url(/a.woff2); font-display: swap; }</style>
            <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter&display=swap">"#,
        );
        assert_eq!(good.message("performance/font-display"), "font-display set on 2 web font declaration(s)");
        let bad = page(
            r#"<style>@font-face { font-family: A; src: url(/a.woff2); }</style>
            <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter">"#,
        );
        assert_eq!(bad.message("performance/font-display"), "2 of 2 web font declaration(s) without font-display");
    }

    #[test]
    fn http_version() {
        assert_eq!(page("<p>x</p>").message("performance/http-version"), "Served over HTTP/2");
        let mut old = page("<p>x</p>");
        old.http_version = Some("HTTP/1.1".into());
        assert_eq!(old.status("performance/http-version"), Status::Warn);
        let offered = old.header("alt-svc", r#"h3=":443"; ma=86400"#);
        assert_eq!(offered.message("performance/http-version"), "Served over HTTP/1.1, and HTTP/3 is offered");
    }

    #[test]
    fn resource_hints() {
        assert_eq!(page(r#"<script src="/app.js"></script>"#).status("performance/resource-hints"), Status::Info);
        let bare = page(r#"<script src="https://cdn.other.net/lib.js"></script>"#);
        let r = bare.results("performance/resource-hints").remove(0);
        assert_eq!((r.status, r.evidence.as_deref()), (Status::Warn, Some("https://cdn.other.net")));
        let hinted = page(
            r#"<link rel="preconnect" href="https://cdn.other.net"><script src="https://cdn.other.net/lib.js"></script>"#,
        );
        assert_eq!(hinted.status("performance/resource-hints"), Status::Pass);
        // Subdomains of the page's own site are not third parties.
        assert_eq!(
            page(r#"<script src="https://cdn.example.com/a.js"></script>"#).status("performance/resource-hints"),
            Status::Info
        );
    }

    #[test]
    fn third_party_scripts() {
        assert_eq!(page("<p>x</p>").status("performance/third-party-scripts"), Status::Info);
        let mut small =
            page(r#"<script src="https://cdn.other.net/a.js"></script><script src="https://x.io/b.js"></script>"#);
        small.link_bytes = vec![("https://cdn.other.net/a.js".into(), 30_000)];
        assert_eq!(
            small.message("performance/third-party-scripts"),
            "2 third-party script(s), 30 KB plus 1 of unknown size"
        );
        assert_eq!(small.status("performance/third-party-scripts"), Status::Pass);
        let mut heavy = page(r#"<script src="https://cdn.other.net/a.js"></script>"#);
        heavy.link_bytes = vec![("https://cdn.other.net/a.js".into(), 800_000)];
        assert_eq!(heavy.status("performance/third-party-scripts"), Status::Warn);
        let unknown = page(r#"<script src="https://x.io/b.js"></script>"#);
        assert_eq!(
            unknown.message("performance/third-party-scripts"),
            "1 third-party script(s), size unknown (the server sent no Content-Length)"
        );
    }
}
