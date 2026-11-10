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
    ]
}

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
    use crate::Status::{Fail, Info, Pass, Warn};
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
}
