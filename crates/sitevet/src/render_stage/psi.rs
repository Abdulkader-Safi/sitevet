//! PageSpeed Insights v5, ported from the TS package's `src/providers/psi.ts`: one Lighthouse
//! run per URL gives lab Core Web Vitals and the accessibility audits.

use std::collections::HashMap;
use std::time::Duration;

use futures::StreamExt;
use serde_json::Value;

use super::{Rendered, collect_reporting};
use crate::engine::Events;
use crate::rules::{AxePass, AxeViolation, PerfMetrics, RenderedData};

const ENDPOINT: &str = "https://www.googleapis.com/pagespeedonline/v5/runPagespeed";
/// Google's limit is about 400 requests per 100 s from third parties; 4 in flight stays polite.
const IN_FLIGHT: usize = 4;
const TIMEOUT: Duration = Duration::from_secs(60);
/// The longest `Retry-After` sitevet waits once after a 429.
const MAX_RETRY_WAIT: Duration = Duration::from_secs(60);

pub(crate) struct Psi {
    key: String,
    max_pages: usize,
    endpoint: String,
    client: reqwest::Client,
}

impl Psi {
    pub(crate) fn new(key: String, max_pages: usize) -> Self {
        // SITEVET_PSI_ENDPOINT points tests at recorded responses. Not for normal use.
        let endpoint = std::env::var("SITEVET_PSI_ENDPOINT").unwrap_or_else(|_| ENDPOINT.to_string());
        let client = reqwest::Client::builder().timeout(TIMEOUT).build().unwrap_or_default();
        Self { key, max_pages, endpoint, client }
    }

    pub(crate) async fn render(&self, urls: &[String], events: &Events) -> Rendered {
        // Futures built up front: a closure inside the stream trips a compiler limit on Send.
        let jobs: Vec<_> = urls.iter().take(self.max_pages).map(|url| self.analyze(url.clone())).collect();
        let total = jobs.len() as u32;
        let stream = futures::stream::iter(jobs).buffer_unordered(IN_FLIGHT);
        let pages = collect_reporting(stream, &mut 0, total, events).await.into_iter().collect::<HashMap<_, _>>();
        Rendered { pages, failed: None }
    }

    async fn analyze(&self, url: String) -> (String, RenderedData) {
        let data = self.analyze_one(&url).await;
        (url, data)
    }

    async fn analyze_one(&self, url: &str) -> RenderedData {
        let failed = |error: String| RenderedData { ok: false, error: Some(error), ..Default::default() };
        let query = [
            ("url", url),
            ("category", "performance"),
            ("category", "accessibility"),
            ("strategy", "mobile"),
            ("key", &self.key),
        ];
        let Ok(endpoint) = url::Url::parse_with_params(&self.endpoint, query) else {
            return failed("the PSI endpoint is not a valid URL".into());
        };
        let send = || self.client.get(endpoint.clone()).send();
        let mut res = match send().await {
            Ok(r) => r,
            // The error text can carry the request URL with the key, so it is not passed on.
            Err(e) => return failed(if e.is_timeout() { "PSI timed out".into() } else { "PSI request failed".into() }),
        };
        if res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let wait = res
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map_or(Duration::from_secs(5), Duration::from_secs)
                .min(MAX_RETRY_WAIT);
            tracing::warn!(wait_s = wait.as_secs(), "PSI rate limit, retrying once");
            tokio::time::sleep(wait).await;
            res = match send().await {
                Ok(r) => r,
                Err(_) => return failed("PSI request failed".into()),
            };
        }
        if !res.status().is_success() {
            return failed(format!("PSI HTTP {}", res.status().as_u16()));
        }
        match res.bytes().await.ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
            Some(v) => from_response(&v),
            None => failed("PSI returned an unreadable response".into()),
        }
    }
}

/// Lighthouse audits to RenderedData.
pub(crate) fn from_response(v: &Value) -> RenderedData {
    let audits = &v["lighthouseResult"]["audits"];
    let num = |id: &str| audits[id]["numericValue"].as_f64().unwrap_or(0.0);
    let metrics = PerfMetrics {
        ttfb_ms: num("server-response-time").round(),
        load_ms: num("interactive").round(),
        lcp_ms: num("largest-contentful-paint").round(),
        cls: (num("cumulative-layout-shift") * 1000.0).round() / 1000.0,
        dom_nodes: num("dom-size").round() as u32,
        requests: num("network-requests").round() as u32,
        transfer_bytes: num("total-byte-weight").round() as u64,
    };
    let mut axe = Vec::new();
    let mut axe_passes = Vec::new();
    let refs =
        v["lighthouseResult"]["categories"]["accessibility"]["auditRefs"].as_array().cloned().unwrap_or_default();
    for r in refs {
        let Some(id) = r["id"].as_str() else { continue };
        let a = &audits[id];
        if a.is_null() || matches!(a["scoreDisplayMode"].as_str(), Some("notApplicable" | "manual")) {
            continue;
        }
        let help = a["title"].as_str().unwrap_or(id).to_string();
        match a["score"].as_f64() {
            Some(1.0) => axe_passes.push(AxePass { id: id.into(), help }),
            Some(0.0) => axe.push(AxeViolation {
                id: id.into(),
                impact: Some("serious".into()),
                description: a["description"].as_str().map_or_else(|| help.clone(), str::to_string),
                help,
                help_url: String::new(),
                nodes: 1,
                sample: None,
            }),
            _ => {}
        }
    }
    RenderedData { ok: true, error: None, axe, axe_passes, metrics: Some(metrics) }
}
