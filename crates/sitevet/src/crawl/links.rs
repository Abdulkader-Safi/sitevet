//! The shared link checker: each distinct URL is checked once per audit, whoever asks.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use futures::future::{BoxFuture, Shared};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use url::Url;

use super::fetch::{Body, Fetched, Fetcher};
use super::normalize;
use super::robots::RobotsGuard;

/// The result of checking one URL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LinkStatus {
    /// Final status in 200 to 299.
    pub ok: bool,
    /// 0 when the request failed.
    pub status: u16,
    pub redirected: bool,
    /// Number of redirects followed.
    pub chain: u32,
    pub error: Option<String>,
}

/// Link URL, as written in [`crate::Link::url`], to its status.
pub type LinkStatusMap = HashMap<String, LinkStatus>;

impl LinkStatus {
    pub(crate) fn from_fetch(res: &Result<Fetched, String>) -> Self {
        match res {
            Ok(f) => Self {
                ok: (200..300).contains(&f.status),
                status: f.status,
                redirected: !f.redirect_chain.is_empty(),
                chain: f.redirect_chain.len() as u32,
                error: None,
            },
            Err(e) => Self { ok: false, status: 0, redirected: false, chain: 0, error: Some(e.clone()) },
        }
    }
}

type Pending = Shared<BoxFuture<'static, LinkStatus>>;

pub(crate) struct LinkChecker {
    fetcher: Arc<Fetcher>,
    robots: Arc<RobotsGuard>,
    origin: url::Origin,
    cache: Mutex<HashMap<String, Pending>>,
}

impl LinkChecker {
    pub(crate) fn new(fetcher: Arc<Fetcher>, robots: Arc<RobotsGuard>, origin: url::Origin) -> Self {
        Self { fetcher, robots, origin, cache: Mutex::new(HashMap::new()) }
    }

    /// Records a status the crawl already has, so the URL is not requested again.
    pub(crate) fn seed(&self, url: &Url, status: LinkStatus) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.entry(normalize(url).into()).or_insert_with(|| futures::future::ready(status).boxed().shared());
    }

    fn check(&self, url: Url) -> Pending {
        let url = normalize(&url);
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(url.to_string())
            .or_insert_with(|| {
                let fetcher = self.fetcher.clone();
                let blocked = url.origin() == self.origin && !self.robots.allowed(&url);
                async move {
                    if blocked {
                        return LinkStatus {
                            ok: false,
                            status: 0,
                            redirected: false,
                            chain: 0,
                            error: Some("blocked by robots.txt".into()),
                        };
                    }
                    // HEAD first, GET when the server does not allow HEAD.
                    let head = fetcher.fetch(&url, Method::HEAD, Body::Skip).await;
                    let res = match &head {
                        Ok(f) if f.status == 405 || f.status == 501 => {
                            fetcher.fetch(&url, Method::GET, Body::Skip).await
                        }
                        _ => head,
                    };
                    LinkStatus::from_fetch(&res)
                }
                .boxed()
                .shared()
            })
            .clone()
    }

    /// Checks every http(s) URL given, each distinct one once, all at the same time. Per-host
    /// and external caps in the fetcher bound what is actually in flight.
    pub(crate) async fn check_all<'a>(&self, urls: impl IntoIterator<Item = &'a str>) -> LinkStatusMap {
        let mut wanted: Vec<&str> = urls.into_iter().collect();
        wanted.sort_unstable();
        wanted.dedup();
        let pending: Vec<(&str, Pending)> =
            wanted.into_iter().filter_map(|u| Url::parse(u).ok().map(|parsed| (u, self.check(parsed)))).collect();
        let statuses = futures::future::join_all(pending.iter().map(|(_, p)| p.clone())).await;
        pending.into_iter().zip(statuses).map(|((u, _), s)| (u.to_string(), s)).collect()
    }
}
