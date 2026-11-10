//! The HTTP side: one client per audit, per-host permits and delay, manual redirects, size caps.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE, HeaderMap, LOCATION, USER_AGENT};
use reqwest::{Method, StatusCode};
use tokio::sync::Semaphore;
use url::Url;

use super::ssrf::{self, GuardedResolver};

/// HTML is cut at 10 MB (NFR-05).
pub(crate) const HTML_CAP: usize = 10 * 1024 * 1024;
pub(crate) const ROBOTS_CAP: usize = 500 * 1024;
pub(crate) const SITEMAP_CAP: usize = 50 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;
/// External hosts share this many requests in flight.
const EXTERNAL_CAP: usize = 20;

pub(crate) struct Fetcher {
    client: reqwest::Client,
    user_agent: String,
    origin: url::Origin,
    per_host: usize,
    delay: Duration,
    block_private: bool,
    hosts: Mutex<HashMap<String, Arc<HostState>>>,
    external: Semaphore,
}

struct HostState {
    permits: Semaphore,
    next_start: Mutex<Instant>,
}

#[derive(Debug)]
pub(crate) struct Fetched {
    pub(crate) final_url: Url,
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) content_type: Option<String>,
    pub(crate) body: Vec<u8>,
    pub(crate) truncated: bool,
    /// Each URL that answered with a redirect.
    pub(crate) redirect_chain: Vec<String>,
    pub(crate) ms: u64,
}

/// What to do with the body of the final response.
#[derive(Clone, Copy)]
pub(crate) enum Body {
    /// Read up to this many decoded bytes.
    Read(usize),
    /// Read up to the cap only when the content type is HTML (or missing).
    Html(usize),
    Skip,
}

impl Fetcher {
    pub(crate) fn new(
        user_agent: String,
        origin: url::Origin,
        per_host: usize,
        delay: Duration,
        timeout: Duration,
        block_private: bool,
    ) -> reqwest::Result<Self> {
        // Decoding is done here, not by reqwest: its decoder drops Content-Encoding, which
        // performance/text-compression needs to see.
        let client = reqwest::Client::builder()
            .no_gzip()
            .no_brotli()
            .no_zstd()
            .no_deflate()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .dns_resolver(Arc::new(GuardedResolver { block_private }))
            .build()?;
        Ok(Self {
            client,
            user_agent,
            origin,
            per_host,
            delay,
            block_private,
            hosts: Mutex::new(HashMap::new()),
            external: Semaphore::new(EXTERNAL_CAP),
        })
    }

    /// Follows up to 10 redirects. One retry after 1 s on a reset connection or a 502, 503 or 504.
    pub(crate) async fn fetch(&self, url: &Url, method: Method, body: Body) -> Result<Fetched, String> {
        let started = Instant::now();
        let mut current = url.clone();
        let mut chain = Vec::new();
        loop {
            let res = self.send_with_retry(&current, method.clone()).await?;
            let status = res.status();
            if status.is_redirection()
                && chain.len() < MAX_REDIRECTS
                && let Some(next) = location(&current, res.headers())
            {
                chain.push(current.to_string());
                current = next;
                continue;
            }
            let headers = header_pairs(res.headers());
            let content_type = res.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(str::to_string);
            let cap = match body {
                Body::Read(cap) => cap,
                Body::Html(cap) if is_html(content_type.as_deref()) => cap,
                Body::Html(_) | Body::Skip => 0,
            };
            let (bytes, truncated) = read_capped(res, cap).await?;
            return Ok(Fetched {
                final_url: current,
                status: status.as_u16(),
                headers,
                content_type,
                body: bytes,
                truncated,
                redirect_chain: chain,
                ms: started.elapsed().as_millis() as u64,
            });
        }
    }

    async fn send_with_retry(&self, url: &Url, method: Method) -> Result<reqwest::Response, String> {
        if self.block_private && ssrf::literal_is_blocked(url) {
            return Err(ssrf::BLOCKED.to_string());
        }
        let first = self.send(url, method.clone()).await;
        let retry = match &first {
            Ok(res) => matches!(
                res.status(),
                StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT
            ),
            Err(e) => is_reset(e),
        };
        if !retry {
            return first.map_err(describe);
        }
        tracing::warn!(url = %redact(url), "retrying once in 1 s");
        tokio::time::sleep(Duration::from_secs(1)).await;
        self.send(url, method).await.map_err(describe)
    }

    async fn send(&self, url: &Url, method: Method) -> reqwest::Result<reqwest::Response> {
        let host = self.host(url);
        let _permit = host.permits.acquire().await;
        let _external = if url.origin() == self.origin { None } else { Some(self.external.acquire().await) };
        self.wait_turn(&host).await;
        let started = Instant::now();
        let res = self
            .client
            .request(method.clone(), url.clone())
            .header(USER_AGENT, &self.user_agent)
            .header(ACCEPT_ENCODING, "gzip, br, zstd, deflate")
            .send()
            .await;
        let status = res.as_ref().map_or(0, |r| r.status().as_u16());
        tracing::debug!(%method, url = %redact(url), status, ms = started.elapsed().as_millis() as u64, "request");
        res
    }

    fn host(&self, url: &Url) -> Arc<HostState> {
        let key = format!("{}:{}", url.host_str().unwrap_or(""), url.port_or_known_default().unwrap_or(0));
        let mut hosts = self.hosts.lock().unwrap_or_else(|e| e.into_inner());
        hosts
            .entry(key)
            .or_insert_with(|| {
                Arc::new(HostState { permits: Semaphore::new(self.per_host), next_start: Mutex::new(Instant::now()) })
            })
            .clone()
    }

    /// Spaces request starts to one host by the configured delay.
    async fn wait_turn(&self, host: &HostState) {
        if self.delay.is_zero() {
            return;
        }
        let wait = {
            let mut next = host.next_start.lock().unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            let start = (*next).max(now);
            *next = start + self.delay;
            start - now
        };
        tokio::time::sleep(wait).await;
    }
}

fn describe(e: reqwest::Error) -> String {
    let mut msg = e.to_string();
    let mut source = std::error::Error::source(&e);
    while let Some(s) = source {
        msg = format!("{msg}: {s}");
        source = s.source();
    }
    if msg.contains(ssrf::BLOCKED) { ssrf::BLOCKED.to_string() } else { msg }
}

fn is_reset(e: &reqwest::Error) -> bool {
    let mut source = std::error::Error::source(e);
    while let Some(s) = source {
        if s.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == std::io::ErrorKind::ConnectionReset) {
            return true;
        }
        source = s.source();
    }
    false
}

fn location(current: &Url, headers: &HeaderMap) -> Option<Url> {
    let loc = headers.get(LOCATION)?.to_str().ok()?;
    let next = current.join(loc).ok()?;
    matches!(next.scheme(), "http" | "https").then_some(next)
}

fn header_pairs(headers: &HeaderMap) -> Vec<(String, String)> {
    headers.iter().map(|(k, v)| (k.as_str().to_string(), String::from_utf8_lossy(v.as_bytes()).into_owned())).collect()
}

pub(crate) fn is_html(content_type: Option<&str>) -> bool {
    content_type.is_none_or(|ct| {
        let ct = ct.to_ascii_lowercase();
        ct.contains("text/html") || ct.contains("application/xhtml")
    })
}

/// Reads up to `cap` decoded bytes. Decoding happens before the cap, so a compressed bomb stops
/// at the cap too.
async fn read_capped(res: reqwest::Response, cap: usize) -> Result<(Vec<u8>, bool), String> {
    use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZlibDecoder, ZstdDecoder};
    use futures::TryStreamExt;
    use tokio::io::{AsyncRead, AsyncReadExt};

    let mut out = Vec::new();
    if cap == 0 {
        return Ok((out, false));
    }
    let encoding =
        res.headers().get(CONTENT_ENCODING).and_then(|v| v.to_str().ok()).unwrap_or("").trim().to_ascii_lowercase();
    let stream = res.bytes_stream().map_err(std::io::Error::other);
    let raw = tokio_util::io::StreamReader::new(stream);
    let mut body: std::pin::Pin<Box<dyn AsyncRead + Send>> = match encoding.as_str() {
        "gzip" | "x-gzip" => Box::pin(GzipDecoder::new(raw)),
        "br" => Box::pin(BrotliDecoder::new(raw)),
        "zstd" => Box::pin(ZstdDecoder::new(raw)),
        "deflate" => Box::pin(ZlibDecoder::new(raw)),
        _ => Box::pin(raw),
    };
    (&mut body)
        .take(cap as u64 + 1)
        .read_to_end(&mut out)
        .await
        .map_err(|e| format!("could not read the body: {e}"))?;
    let truncated = out.len() > cap;
    out.truncate(cap);
    Ok((out, truncated))
}

/// The URL without its query string, for logs.
fn redact(url: &Url) -> String {
    let mut u = url.clone();
    u.set_query(None);
    u.to_string()
}
