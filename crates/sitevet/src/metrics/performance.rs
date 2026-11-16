//! Performance depth (FR-PERF-01 to FR-PERF-04): the timings the rendered stage measures, INP
//! with an explicit "not measurable" state, layout shifts, slow requests, and the static font,
//! protocol and third-party numbers behind the performance rules.

use std::sync::Arc;

use super::{cell, cell_n, num, txt, value};
use crate::report::ValueStatus;
use crate::rules::performance as perf;
use crate::{Collector, CollectorMeta, MetricInput, RuleCx, Stage};

/// What the INP metric says when nothing interacted with the page (FR-PERF-02).
pub(crate) const INP_NOT_MEASURABLE: &str = "not measurable without interaction";

pub(super) fn collectors() -> Vec<Arc<dyn Collector>> {
    vec![Arc::new(Timings), Arc::new(Static), Arc::new(ThirdParty)]
}

fn ms(id: &str, label: &str, v: Option<f64>) -> MetricInput {
    MetricInput::value(id, label, v.and_then(num), Some("ms"))
}

pub(crate) struct Timings;

impl Collector for Timings {
    fn meta(&self) -> CollectorMeta {
        CollectorMeta::new("performance/timings", "performance", Stage::Rendered)
    }

    fn collect(&self, cx: &RuleCx<'_>) -> Vec<MetricInput> {
        let Some(m) = cx.rendered.filter(|r| r.ok).and_then(|r| r.metrics.as_ref()) else { return Vec::new() };
        let t = m.timings.as_ref();
        let not_reported = "Not reported by PageSpeed Insights";
        let phase = |id: &str, label: &str, f: fn(&crate::NavTimings) -> f64, note: &str| {
            ms(id, label, t.map(f)).with_note(if t.is_some() { note } else { not_reported })
        };
        let mut out = vec![
            ms("performance/load-time", "Total load time", Some(m.load_ms)).with_note("Navigation start to load end"),
            phase("performance/dns-time", "DNS lookup", |t| t.dns_ms, "0 when the lookup was cached"),
            phase("performance/tcp-time", "TCP connection", |t| t.tcp_ms, "Includes TLS. 0 on a reused connection"),
            phase(
                "performance/response-time",
                "Response time",
                |t| t.response_ms,
                "First byte to last byte of the HTML",
            ),
            phase("performance/dom-interactive", "DOM interactive", |t| t.dom_interactive_ms, "Since navigation start"),
            phase(
                "performance/dom-content-loaded",
                "DOM content loaded",
                |t| t.dom_content_loaded_ms,
                "Since navigation start",
            ),
            phase("performance/dom-complete", "DOM complete", |t| t.dom_complete_ms, "Since navigation start"),
            ms("performance/fcp", "First contentful paint", m.fcp_ms)
                .with_status(m.fcp_ms.map_or(ValueStatus::Missing, |f| super::up_to(f, 1800.0, 3000.0)))
                .with_note("1.8 s or less is good"),
        ];
        out.push(match m.inp_ms {
            Some(inp) => ms("performance/inp", "Interaction to Next Paint", Some(inp))
                .with_status(super::up_to(inp, 200.0, 500.0))
                .with_note("200 ms or less is good"),
            None => value("performance/inp", "Interaction to Next Paint", txt(INP_NOT_MEASURABLE))
                .with_status(ValueStatus::Missing)
                .with_note("INP needs a real click, tap or key press. Field data (CrUX) has it for busy pages"),
        });
        if let (Some(n), Some(largest)) = (m.layout_shifts, m.largest_shift) {
            out.push(
                value("performance/layout-shifts", "Layout shifts", num(n)).with_note("Shifts without recent input"),
            );
            out.push(
                value("performance/largest-shift", "Largest single shift", num(largest))
                    .with_status(super::up_to(largest, 0.05, 0.1)),
            );
        }
        if let Some(slow) = &m.slow_resources {
            out.push(
                value("performance/slow-resources", "Slow requests", num(slow.len() as f64))
                    .with_status(if slow.is_empty() { ValueStatus::Ok } else { ValueStatus::Warn })
                    .with_note(if slow.is_empty() {
                        "None exceeded the threshold".to_string()
                    } else {
                        "Requests that took 300 ms or more by default (slow_resource_ms)".to_string()
                    }),
            );
            if !slow.is_empty() {
                let mut rows: Vec<_> = slow.iter().collect();
                rows.sort_by(|a, b| b.ms.total_cmp(&a.ms));
                out.push(MetricInput::table(
                    "performance/slow-resource-list",
                    "Slow requests",
                    &["URL", "Type", "ms"],
                    rows.iter().map(|r| vec![cell(&r.url), cell(&r.kind), cell_n(r.ms)]).collect(),
                ));
            }
        }
        out
    }
}

dom_collector!(Static, "performance", "performance/static", |cx, dom, html| {
    let (fonts, missing_display) = perf::font_display(dom);
    let (files, preloaded) = perf::font_files(dom);
    let version = cx.page.http_version.clone();
    let modern = version.as_deref().is_some_and(|v| v == "HTTP/2" || v == "HTTP/3");
    vec![
        value("performance/web-fonts", "Web font declarations", num(fonts as f64)).with_note(format!(
            "{missing_display} without font-display. Inline @font-face and Google Fonts links only"
        )),
        value("performance/font-files", "Font files", num(files as f64))
            .with_note("Referenced from inline CSS. The rendered stage lists slow font requests from any stylesheet"),
        value("performance/font-preloads", "Preloaded fonts", num(preloaded as f64)),
        value("performance/http-version", "HTTP version", version.clone().and_then(txt)).with_status(match version {
            None => ValueStatus::Missing,
            Some(_) if modern => ValueStatus::Ok,
            Some(_) => ValueStatus::Warn,
        }),
    ]
});

pub(crate) struct ThirdParty;

impl Collector for ThirdParty {
    fn meta(&self) -> CollectorMeta {
        CollectorMeta::new("performance/third-party", "performance", Stage::Enriched)
    }

    fn collect(&self, cx: &RuleCx<'_>) -> Vec<MetricInput> {
        let scripts = perf::third_party_scripts(cx);
        if scripts.is_empty() {
            return Vec::new();
        }
        vec![MetricInput::table(
            "performance/third-party-scripts",
            "Third-party scripts",
            &["URL", "KB"],
            scripts
                .iter()
                .map(|(u, b)| vec![cell(u), b.map_or_else(|| cell("unknown"), |b| cell_n(b.div_ceil(1024) as f64))])
                .collect(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::INP_NOT_MEASURABLE;
    use crate::report::ValueStatus::{Missing, Ok, Warn};
    use crate::rules::testkit::{Page, n, page, row, rows, t, value};
    use crate::{NavTimings, PerfMetrics, RenderedData, SlowResource};

    fn rendered(m: PerfMetrics) -> Page {
        let mut p = page("<p>x</p>");
        p.rendered = Some(RenderedData { ok: true, metrics: Some(m), ..Default::default() });
        p
    }

    fn browser_metrics() -> PerfMetrics {
        PerfMetrics {
            load_ms: 900.0,
            timings: Some(NavTimings {
                dns_ms: 12.0,
                tcp_ms: 30.0,
                response_ms: 8.0,
                dom_interactive_ms: 300.0,
                dom_content_loaded_ms: 320.0,
                dom_complete_ms: 880.0,
            }),
            fcp_ms: Some(250.0),
            layout_shifts: Some(2),
            largest_shift: Some(0.08),
            slow_resources: Some(vec![SlowResource {
                url: "https://example.com/hero.jpg".into(),
                ms: 640.0,
                kind: "img".into(),
            }]),
            ..Default::default()
        }
    }

    #[test]
    fn browser_timings_inp_shifts_and_slow_requests() {
        let p = rendered(browser_metrics());
        for (id, v) in [
            ("performance/load-time", 900.0),
            ("performance/dns-time", 12.0),
            ("performance/tcp-time", 30.0),
            ("performance/response-time", 8.0),
            ("performance/dom-interactive", 300.0),
            ("performance/dom-content-loaded", 320.0),
            ("performance/dom-complete", 880.0),
            ("performance/fcp", 250.0),
        ] {
            assert_eq!(value(&p.metric(id)).0, n(v), "{id}");
        }
        // FR-PERF-02: no interaction gives an explicit state, never 0 or empty.
        let inp = value(&p.metric("performance/inp"));
        assert_eq!((inp.0, inp.1), (t(INP_NOT_MEASURABLE), Some(Missing)));
        assert_eq!(value(&p.metric("performance/layout-shifts")).0, n(2.0));
        assert_eq!(value(&p.metric("performance/largest-shift")).1, Some(Warn));
        assert_eq!(value(&p.metric("performance/slow-resources")).0, n(1.0));
        assert_eq!(
            rows(&p.metric("performance/slow-resource-list")),
            [row(&["https://example.com/hero.jpg", "img", "640"])]
        );
    }

    #[test]
    fn none_slow_and_a_measured_inp() {
        let mut m = browser_metrics();
        m.slow_resources = Some(Vec::new());
        m.inp_ms = Some(120.0);
        let p = rendered(m);
        let slow = value(&p.metric("performance/slow-resources"));
        assert_eq!((slow.0, slow.1, slow.2.as_deref()), (n(0.0), Some(Ok), Some("None exceeded the threshold")));
        assert!(p.metrics().iter().all(|m| m.id != "performance/slow-resource-list"));
        assert_eq!(value(&p.metric("performance/inp")).0, n(120.0));
    }

    #[test]
    fn pagespeed_has_fewer_numbers() {
        let p = rendered(PerfMetrics { load_ms: 2000.0, fcp_ms: Some(1500.0), ..Default::default() });
        let dns = value(&p.metric("performance/dns-time"));
        assert_eq!((dns.0, dns.2.as_deref()), (None, Some("Not reported by PageSpeed Insights")));
        assert_eq!(value(&p.metric("performance/fcp")).0, n(1500.0));
        assert!(p.metrics().iter().all(|m| m.id != "performance/layout-shifts"));
        // Without the rendered stage there are no timings at all.
        assert!(page("<p>x</p>").metrics().iter().all(|m| m.id != "performance/load-time"));
    }

    #[test]
    fn static_numbers() {
        let p = page(
            r#"<style>@font-face { font-family: A; src: url(/a.woff2), url(/a.woff); }</style>
            <link rel="preload" as="font" href="/a.woff2" crossorigin>"#,
        );
        assert_eq!(value(&p.metric("performance/web-fonts")).0, n(1.0));
        assert_eq!(value(&p.metric("performance/font-files")).0, n(2.0));
        assert_eq!(value(&p.metric("performance/font-preloads")).0, n(1.0));
        assert_eq!(value(&p.metric("performance/http-version")).0, t("HTTP/2"));
    }
}
