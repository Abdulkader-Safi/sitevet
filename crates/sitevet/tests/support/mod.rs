//! Fixture server for integration tests: serves `tests/fixtures/sites/<name>/` or generated
//! pages through wiremock, logs every request, and can count open connections through a proxy.
#![allow(dead_code, clippy::unwrap_used)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sitevet::{
    Collector, CollectorMeta, MetricCell, MetricInput, Rule, RuleCx, RuleMeta, RuleResult, Severity, Stage, Status,
};
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

#[derive(Clone)]
pub struct Route {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub delay: Duration,
}

impl Route {
    pub fn html(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".into(), "text/html; charset=utf-8".into())],
            body: body.into().into_bytes(),
            delay: Duration::ZERO,
        }
    }

    pub fn redirect(to: &str) -> Self {
        Self { status: 301, headers: vec![("location".into(), to.into())], body: Vec::new(), delay: Duration::ZERO }
    }

    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn content_type(mut self, ct: &str) -> Self {
        self.headers.retain(|(k, _)| k != "content-type");
        self.headers.push(("content-type".into(), ct.into()));
        self
    }
}

/// Path to route. Bodies may use `{{origin}}`, replaced with the server's base URL.
#[derive(Clone, Default)]
pub struct Site {
    routes: HashMap<String, Route>,
}

#[derive(serde::Deserialize)]
struct SiteToml {
    #[serde(default)]
    routes: HashMap<String, RouteToml>,
}

#[derive(serde::Deserialize)]
struct RouteToml {
    status: Option<u16>,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: Option<String>,
    delay_ms: Option<u64>,
}

impl Site {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every file under `tests/fixtures/sites/<name>/` at its own path, `index.html` also at its
    /// folder's path, then the overrides in `site.toml`.
    pub fn from_dir(name: &str) -> Self {
        // Works from either crate: both sit in crates/, and the fixtures live in crates/sitevet.
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let root = crates.join("sitevet/tests/fixtures/sites").join(name);
        let mut site = Self::new();
        site.add_dir(&root, &root);
        if let Ok(text) = std::fs::read_to_string(root.join("site.toml")) {
            let parsed: SiteToml = toml::from_str(&text).unwrap();
            for (path, r) in parsed.routes {
                let mut route = site.routes.remove(&path).unwrap_or_else(|| Route::html(""));
                route.status = r.status.unwrap_or(route.status);
                for (k, v) in r.headers {
                    route.headers.retain(|(name, _)| !name.eq_ignore_ascii_case(&k));
                    route.headers.push((k.to_lowercase(), v));
                }
                if let Some(body) = r.body {
                    route.body = body.into_bytes();
                }
                route.delay = Duration::from_millis(r.delay_ms.unwrap_or(0));
                site.routes.insert(path, route);
            }
        }
        site
    }

    fn add_dir(&mut self, root: &Path, dir: &Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                self.add_dir(root, &path);
                continue;
            }
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if rel == "site.toml" {
                continue;
            }
            let ct = match path.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("xml") => "application/xml",
                Some("txt") => "text/plain",
                _ => "application/octet-stream",
            };
            let route = Route::html("").content_type(ct);
            let route = Route { body: std::fs::read(&path).unwrap(), ..route };
            if let Some(folder) = rel.strip_suffix("index.html") {
                self.routes.insert(format!("/{folder}"), route.clone());
            }
            self.routes.insert(format!("/{rel}"), route);
        }
    }

    pub fn page(self, path: &str, html: impl Into<String>) -> Self {
        self.route(path, Route::html(html))
    }

    pub fn route(mut self, path: &str, route: Route) -> Self {
        self.routes.insert(path.into(), route);
        self
    }
}

#[derive(Clone, Debug)]
pub struct Hit {
    pub method: String,
    pub path: String,
    pub at: Instant,
    pub user_agent: String,
}

pub type Log = Arc<Mutex<Vec<Hit>>>;

struct Responder {
    site: Site,
    origin: String,
    log: Log,
}

impl Respond for Responder {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        let path = req.url.path().to_string();
        let user_agent = req.headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        self.log.lock().unwrap().push(Hit {
            method: req.method.to_string(),
            path: path.clone(),
            at: Instant::now(),
            user_agent,
        });
        let Some(route) = self.site.routes.get(&path) else {
            return ResponseTemplate::new(404).set_body_raw("<h1>Not found</h1>", "text/html");
        };
        // Text bodies get {{origin}} filled in; binary ones (compressed) go out as they are.
        let body = match std::str::from_utf8(&route.body) {
            Ok(text) => text.replace("{{origin}}", &self.origin).into_bytes(),
            Err(_) => route.body.clone(),
        };
        let mut res = ResponseTemplate::new(route.status).set_body_bytes(body).set_delay(route.delay);
        for (k, v) in &route.headers {
            res = res.insert_header(k.as_str(), v.as_str());
        }
        res
    }
}

pub struct Fixture {
    pub server: MockServer,
    pub log: Log,
}

impl Fixture {
    pub async fn start(site: Site) -> Self {
        let server = MockServer::start().await;
        let log = Log::default();
        let responder = Responder { site, origin: server.uri(), log: log.clone() };
        Mock::given(any()).respond_with(responder).mount(&server).await;
        Self { server, log }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.server.uri())
    }

    pub fn hits(&self) -> Vec<Hit> {
        self.log.lock().unwrap().clone()
    }

    pub fn count(&self, path: &str) -> usize {
        self.hits().iter().filter(|h| h.path == path).count()
    }
}

/// A TCP proxy in front of `target` that counts client connections still open.
pub async fn counting_proxy(target: SocketAddr) -> (String, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let open = Arc::new(AtomicUsize::new(0));
    let counter = open.clone();
    tokio::spawn(async move {
        while let Ok((client, _)) = listener.accept().await {
            counter.fetch_add(1, Ordering::SeqCst);
            let open = counter.clone();
            tokio::spawn(async move {
                let Ok(server) = tokio::net::TcpStream::connect(target).await else {
                    open.fetch_sub(1, Ordering::SeqCst);
                    return;
                };
                let (mut cr, mut cw) = client.into_split();
                let (mut sr, mut sw) = server.into_split();
                let down = tokio::spawn(async move { tokio::io::copy(&mut sr, &mut cw).await });
                // The client closing its side is what counts.
                let _ = tokio::io::copy(&mut cr, &mut sw).await;
                open.fetch_sub(1, Ordering::SeqCst);
                down.abort();
            });
        }
    });
    (format!("http://{addr}"), open)
}

/// A fixture and counting proxy on their own thread and runtime, for tests that must not have
/// a tokio context. Shuts down on drop.
pub struct ThreadFixture {
    pub url: String,
    pub proxy_url: String,
    pub open: Arc<AtomicUsize>,
    pub log: Log,
    stop: std::sync::mpsc::Sender<()>,
}

impl ThreadFixture {
    pub fn start(site: Site) -> Self {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (stop, stop_rx) = std::sync::mpsc::channel::<()>();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
            rt.block_on(async move {
                let fixture = Fixture::start(site).await;
                let (proxy_url, open) = counting_proxy(*fixture.server.address()).await;
                ready_tx.send((fixture.url(""), proxy_url, open, fixture.log.clone())).unwrap();
                let _ = tokio::task::spawn_blocking(move || stop_rx.recv()).await;
                drop(fixture);
            });
        });
        let (url, proxy_url, open, log) = ready_rx.recv().unwrap();
        Self { url, proxy_url, open, log, stop }
    }
}

impl Drop for ThreadFixture {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

/// `n` pages: `/` plus `/p1` to `/p{n-1}`, every page linking to all others is too many links,
/// so each page links to the next 5.
pub fn chain_site(n: usize, delay: Duration) -> Site {
    let path = |i: usize| if i == 0 { "/".to_string() } else { format!("/p{i}") };
    let mut site = Site::new();
    for i in 0..n {
        let links: String = (i + 1..(i + 6).min(n)).map(|j| format!("<a href=\"{}\">{j}</a>", path(j))).collect();
        site = site.route(
            &path(i),
            Route::html(format!("<html><head><title>Page {i}</title></head><body>{links}</body></html>")).delay(delay),
        );
    }
    site
}

/// Passes when the page has a title, fails otherwise. DOM stage.
pub struct TitleRule;

impl Rule for TitleRule {
    fn meta(&self) -> RuleMeta {
        RuleMeta::new("test/title", "test", "Has a title", Severity::Error, Stage::Dom)
    }

    fn run(&self, cx: &RuleCx<'_>) -> Vec<RuleResult> {
        assert!(cx.dom.is_some());
        match &cx.page.title {
            Some(_) => vec![RuleResult::new(Status::Pass, "title present")],
            None => vec![RuleResult::new(Status::Fail, "no title")],
        }
    }
}

/// Fails once per broken link. Enriched stage, so the link checker runs.
pub struct BrokenLinksRule;

impl Rule for BrokenLinksRule {
    fn meta(&self) -> RuleMeta {
        RuleMeta::new("test/broken-links", "links", "No broken links", Severity::Warning, Stage::Enriched)
    }

    fn run(&self, cx: &RuleCx<'_>) -> Vec<RuleResult> {
        let broken: Vec<&str> = cx.links.iter().filter(|(_, s)| !s.ok).map(|(u, _)| u.as_str()).collect();
        if broken.is_empty() {
            return vec![RuleResult::new(Status::Pass, "no broken links")];
        }
        broken
            .into_iter()
            .map(|u| {
                let mut r = RuleResult::new(Status::Fail, "broken link");
                r.evidence = Some(u.to_string());
                r
            })
            .collect()
    }
}

/// Word count of the page text. DOM stage.
pub struct WordCount;

impl Collector for WordCount {
    fn meta(&self) -> CollectorMeta {
        CollectorMeta::new("test/word-count", "test", Stage::Dom)
    }

    fn collect(&self, cx: &RuleCx<'_>) -> Vec<MetricInput> {
        let words = cx.page.text.split_whitespace().count();
        vec![MetricInput::value("test/word-count", "Words", Some(MetricCell::Number(words as f64)), None)]
    }
}

pub fn test_options() -> sitevet::Options {
    let mut o = sitevet::Options::default();
    o.rules = Some(vec![Arc::new(TitleRule), Arc::new(BrokenLinksRule)]);
    o.collectors = Some(vec![Arc::new(WordCount)]);
    o
}
