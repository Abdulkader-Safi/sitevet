//! The crawl: robots.txt and sitemaps once per site, then a breadth-first queue of same-origin pages.

mod fetch;
mod links;
mod robots;
mod sitemap;
mod ssrf;

use std::collections::{HashSet, VecDeque};
use std::future::Future;

use futures::StreamExt;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use url::Url;

use fetch::{Body, HTML_CAP, ROBOTS_CAP, SITEMAP_CAP};
pub(crate) use fetch::{Fetcher, decode_html};
pub(crate) use links::LinkChecker;
pub(crate) use links::ROBOTS_BLOCKED;
pub use links::{LinkStatus, LinkStatusMap};
pub(crate) use robots::RobotsGuard;
pub use robots::RobotsInfo;
pub use sitemap::SitemapInfo;
pub(crate) use ssrf::host_is_private;

use crate::engine::{AuditEvent, Events};
use crate::report::BlockedUrl;
use crate::{Error, PageData, Result};

/// What every rule can read about the site: fetched once per audit.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SiteData {
    /// `https://example.com`
    pub origin: String,
    pub start_url: String,
    pub robots: RobotsInfo,
    pub sitemap: SitemapInfo,
}

/// Drops the fragment. `/a` and `/a/` stay distinct here; the crawl merges them when one
/// redirects to the other.
pub(crate) fn normalize(url: &Url) -> Url {
    let mut u = url.clone();
    u.set_fragment(None);
    u
}

/// Fetches robots.txt and every sitemap it lists (or `/sitemap.xml`), each file once.
pub(crate) async fn load_site(fetcher: &Fetcher, start: &Url) -> SiteData {
    let mut site =
        SiteData { origin: start.origin().ascii_serialization(), start_url: start.to_string(), ..Default::default() };
    if let Ok(robots_url) = start.join("/robots.txt") {
        match fetcher.fetch(&robots_url, Method::GET, Body::Read(ROBOTS_CAP)).await {
            Ok(f) if (200..300).contains(&f.status) => {
                let content = String::from_utf8_lossy(&f.body).into_owned();
                site.robots.sitemaps = content
                    .lines()
                    .filter_map(|l| {
                        let (key, value) = l.split_once(':')?;
                        key.trim().eq_ignore_ascii_case("sitemap").then(|| value.trim().to_string())
                    })
                    .collect();
                site.robots = RobotsInfo { exists: true, status: f.status, content, sitemaps: site.robots.sitemaps };
            }
            Ok(f) => site.robots.status = f.status,
            Err(_) => {}
        }
    }

    let mut queue: VecDeque<Url> = site.robots.sitemaps.iter().filter_map(|s| start.join(s).ok()).collect();
    if queue.is_empty() {
        queue.extend(start.join("/sitemap.xml").ok());
    }
    let mut fetched = HashSet::new();
    let mut first = true;
    while let Some(file) = queue.pop_front() {
        if site.sitemap.urls.len() >= sitemap::URL_CAP || !fetched.insert(file.to_string()) {
            continue;
        }
        let res = fetcher.fetch(&file, Method::GET, Body::Read(SITEMAP_CAP)).await;
        let ok = matches!(&res, Ok(f) if (200..300).contains(&f.status));
        if first {
            first = false;
            site.sitemap.exists = ok;
            site.sitemap.status = res.as_ref().map_or(0, |f| f.status);
        }
        let Ok(f) = res else { continue };
        if !ok {
            continue;
        }
        let parsed = sitemap::parse(&f.body);
        if parsed.is_index {
            queue.extend(parsed.locs.iter().filter_map(|l| Url::parse(l).ok()));
        } else {
            let room = sitemap::URL_CAP - site.sitemap.urls.len();
            site.sitemap.urls.extend(parsed.locs.into_iter().take(room));
        }
    }
    site
}

pub(crate) struct Crawl<'a> {
    pub(crate) fetcher: &'a Fetcher,
    pub(crate) robots: &'a RobotsGuard,
    pub(crate) links: &'a LinkChecker,
    pub(crate) start: &'a Url,
    pub(crate) site: &'a SiteData,
    pub(crate) max_pages: usize,
    pub(crate) max_depth: u32,
    pub(crate) concurrency: usize,
    pub(crate) crawl_sitemap: bool,
}

pub(crate) struct Crawled<T> {
    pub(crate) pages: Vec<(PageData, T)>,
    pub(crate) blocked: Vec<BlockedUrl>,
}

enum Outcome<T> {
    Page(Box<PageData>, T, u64),
    NotHtml { url: String, content_type: String },
}

/// URLs waiting to be fetched, plus everything seen and blocked so far.
struct Frontier<'a> {
    robots: &'a RobotsGuard,
    seen: HashSet<String>,
    queue: VecDeque<(Url, u32)>,
    blocked: Vec<BlockedUrl>,
}

impl Frontier<'_> {
    fn push(&mut self, url: &Url, depth: u32) {
        let url = normalize(url);
        if !self.seen.insert(url.to_string()) {
            return;
        }
        if self.robots.allowed(&url) {
            self.queue.push_back((url, depth));
        } else {
            self.blocked.push(BlockedUrl { rule: self.robots.matching_rule(&url), url: url.into() });
        }
    }
}

impl Crawl<'_> {
    /// Breadth first from the start URL, then sitemap URLs at depth 1, up to `max_pages`.
    /// `handle` turns a fetched page and its body into PageData with links, plus the caller's
    /// per-page output. An unreachable or non-HTML start page is an error; any other failure
    /// becomes a page with `error` set.
    pub(crate) async fn run<T, F, Fut>(&self, events: &Events, handle: F) -> Result<Crawled<T>>
    where
        F: Fn(PageData, Vec<u8>) -> Fut,
        Fut: Future<Output = (PageData, T)>,
    {
        let mut frontier =
            Frontier { robots: self.robots, seen: HashSet::new(), queue: VecDeque::new(), blocked: Vec::new() };
        frontier.push(self.start, 0);
        if self.crawl_sitemap {
            let origin = self.start.origin();
            for u in self.site.sitemap.urls.iter().filter_map(|u| Url::parse(u).ok()) {
                if u.origin() == origin {
                    frontier.push(&u, 1);
                }
            }
        }

        let mut pages: Vec<(PageData, T)> = Vec::new();
        let mut audited = HashSet::new();
        while !frontier.queue.is_empty() && pages.len() < self.max_pages {
            let take = (self.max_pages - pages.len()).min(frontier.queue.len());
            let batch: Vec<(Url, u32)> = frontier.queue.drain(..take).collect();
            let mut results = futures::stream::iter(batch)
                .map(|(url, depth)| self.fetch_page(url, depth, &handle))
                .buffered(self.concurrency);
            while let Some(outcome) = results.next().await {
                let (page, extra, ms) = match outcome {
                    Outcome::Page(page, extra, ms) => (*page, extra, ms),
                    Outcome::NotHtml { url, content_type } if url == normalize(self.start).as_str() => {
                        return Err(Error::NotHtml { url, content_type });
                    }
                    Outcome::NotHtml { .. } => continue,
                };
                if page.depth == 0
                    && let Some(e) = &page.error
                {
                    return Err(Error::StartPageUnreachable { url: page.url.clone(), source: e.clone().into() });
                }
                let final_key =
                    Url::parse(&page.final_url).map_or_else(|_| page.final_url.clone(), |u| normalize(&u).into());
                if !audited.insert(final_key.clone()) {
                    continue; // reached through a redirect from another URL already audited
                }
                frontier.seen.insert(final_key);
                events
                    .send(AuditEvent::PageFetched {
                        url: page.url.clone(),
                        status: page.status,
                        depth: page.depth,
                        index: pages.len() as u32 + 1,
                        ms,
                    })
                    .await;
                if page.depth < self.max_depth {
                    for link in page.links.iter().filter(|l| l.internal) {
                        if let Some(u) = link.url.as_deref().and_then(|u| Url::parse(u).ok()) {
                            frontier.push(&u, page.depth + 1);
                        }
                    }
                }
                pages.push((page, extra));
            }
        }
        Ok(Crawled { pages, blocked: frontier.blocked })
    }

    async fn fetch_page<T, F, Fut>(&self, url: Url, depth: u32, handle: &F) -> Outcome<T>
    where
        F: Fn(PageData, Vec<u8>) -> Fut,
        Fut: Future<Output = (PageData, T)>,
    {
        let res = self.fetcher.fetch(&url, Method::GET, Body::Html(HTML_CAP)).await;
        let status = LinkStatus::from_fetch(&res);
        self.links.seed(&url, status.clone());
        let mut page = PageData { url: url.to_string(), final_url: url.to_string(), depth, ..Default::default() };
        let body = match res {
            Ok(f) => {
                self.links.seed(&f.final_url, status);
                if !fetch::is_html(f.content_type.as_deref()) {
                    return Outcome::NotHtml { url: page.url, content_type: f.content_type.unwrap_or_default() };
                }
                page.final_url = f.final_url.into();
                page.status = f.status;
                page.redirect_chain = f.redirect_chain;
                page.response_ms = f.ms;
                page.headers = f.headers;
                page.html_bytes = f.body.len() as u64;
                page.truncated = f.truncated;
                f.body
            }
            Err(e) => {
                page.error = Some(e);
                Vec::new()
            }
        };
        let ms = page.response_ms;
        let (page, extra) = handle(page, body).await;
        Outcome::Page(Box::new(page), extra, ms)
    }
}
