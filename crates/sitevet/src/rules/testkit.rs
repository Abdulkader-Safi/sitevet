//! Runs one rule or collector on an HTML snippet, for the per-category tests.

use std::sync::Arc;

use scraper::Html;

use super::{Collector, RenderedData, Rule, RuleCx};
use crate::report::{MetricBody, MetricCell, Status, ValueStatus};
use crate::{LinkStatus, LinkStatusMap, PageData, RobotsInfo, RuleResult, SiteData, SitemapInfo};

pub(crate) struct Page {
    pub html: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub status: u16,
    pub redirects: usize,
    pub site: SiteData,
    /// Status for every link not listed in `link_status`.
    pub links_ok: bool,
    pub link_status: Vec<(String, u16)>,
    pub rendered: Option<RenderedData>,
}

/// A TS-style fixture page at `https://example.com/x`, with robots.txt and a sitemap.
pub(crate) fn page(html: &str) -> Page {
    Page {
        html: html.into(),
        url: "https://example.com/x".into(),
        headers: Vec::new(),
        status: 200,
        redirects: 0,
        site: SiteData {
            origin: "https://example.com".into(),
            start_url: "https://example.com/".into(),
            robots: RobotsInfo {
                exists: true,
                status: 200,
                content: "User-agent: *\nSitemap: https://example.com/sitemap.xml".into(),
                sitemaps: vec!["https://example.com/sitemap.xml".into()],
            },
            sitemap: SitemapInfo { exists: true, status: 200, urls: vec!["https://example.com/".into()] },
            llms: Default::default(),
        },
        links_ok: true,
        link_status: Vec::new(),
        rendered: None,
    }
}

/// One of the TS package's fixtures, from `tests/fixtures/pages/`.
pub(crate) fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages").join(name);
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{name}: {e}"))
}

impl Page {
    pub fn url(mut self, url: &str) -> Self {
        self.url = url.into();
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    fn with_cx<T>(&self, f: impl FnOnce(&RuleCx<'_>) -> T) -> T {
        let dom = Html::parse_document(&self.html);
        let mut data = PageData {
            url: self.url.clone(),
            final_url: self.url.clone(),
            status: self.status,
            headers: self.headers.clone(),
            redirect_chain: vec!["https://example.com/hop".into(); self.redirects],
            html_bytes: self.html.len() as u64,
            response_ms: 12,
            ..Default::default()
        };
        let origin = url::Url::parse(&self.site.origin)
            .map(|u| u.origin())
            .unwrap_or_else(|_| url::Url::parse("https://example.com").unwrap().origin());
        crate::extract::extract(&dom, &mut data, &origin);
        let mut links = LinkStatusMap::new();
        for l in data.links.iter().filter_map(|l| l.url.clone()) {
            let status = self.link_status.iter().find(|(u, _)| *u == l).map(|(_, s)| *s).unwrap_or(if self.links_ok {
                200
            } else {
                404
            });
            links.insert(
                l,
                LinkStatus { ok: (200..300).contains(&status), status, redirected: false, chain: 0, error: None },
            );
        }
        let cx = RuleCx {
            page: &data,
            dom: Some(&dom),
            html: Some(&self.html),
            rendered: self.rendered.as_ref(),
            links: &links,
            site: &self.site,
        };
        f(&cx)
    }

    pub fn run(&self, rule: &dyn Rule) -> Vec<RuleResult> {
        self.with_cx(|cx| rule.run(cx))
    }

    /// Statuses of the rule with this ID from the registry.
    pub fn statuses(&self, id: &str) -> Vec<Status> {
        self.results(id).iter().map(|r| r.status).collect()
    }

    pub fn results(&self, id: &str) -> Vec<RuleResult> {
        let rule = rule(id);
        self.run(rule.as_ref())
    }

    pub fn status(&self, id: &str) -> Status {
        let s = self.statuses(id);
        assert_eq!(s.len(), 1, "{id} gave {s:?}");
        s[0]
    }

    pub fn message(&self, id: &str) -> String {
        self.results(id).remove(0).message
    }

    pub fn metrics(&self) -> Vec<crate::Metric> {
        self.with_cx(|cx| {
            super::all_collectors()
                .iter()
                .flat_map(|c: &Arc<dyn Collector>| {
                    let cat = c.meta().category;
                    c.collect(cx).into_iter().map(move |m| crate::Metric {
                        id: m.id,
                        category: cat.into(),
                        label: m.label,
                        status: m.status,
                        note: m.note,
                        body: m.body,
                    })
                })
                .collect()
        })
    }

    pub fn metric(&self, id: &str) -> crate::Metric {
        self.metrics().into_iter().find(|m| m.id == id).unwrap_or_else(|| panic!("no metric {id}"))
    }
}

pub(crate) fn rule(id: &str) -> Arc<dyn Rule> {
    super::all_rules().into_iter().find(|r| r.meta().id == id).unwrap_or_else(|| panic!("no rule {id}"))
}

/// (value, status, note) of a value metric.
pub(crate) fn value(m: &crate::Metric) -> (Option<MetricCell>, Option<ValueStatus>, Option<String>) {
    match &m.body {
        MetricBody::Value { value, .. } => (value.clone(), m.status, m.note.clone()),
        _ => panic!("{} is not a value", m.id),
    }
}

pub(crate) fn rows(m: &crate::Metric) -> Vec<Vec<MetricCell>> {
    match &m.body {
        MetricBody::Table { rows, .. } => rows.clone(),
        _ => panic!("{} is not a table", m.id),
    }
}

pub(crate) fn n(v: f64) -> Option<MetricCell> {
    Some(MetricCell::Number(v))
}

pub(crate) fn t(v: &str) -> Option<MetricCell> {
    Some(MetricCell::Text(v.into()))
}

pub(crate) fn row(cells: &[&str]) -> Vec<MetricCell> {
    cells
        .iter()
        .map(|c| c.parse::<f64>().map_or_else(|_| MetricCell::Text(c.to_string()), MetricCell::Number))
        .collect()
}
