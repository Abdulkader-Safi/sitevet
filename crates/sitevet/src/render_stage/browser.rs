//! A local Chrome through chromiumoxide: a fresh profile per audit, 2 tabs at a time, axe-core
//! and Core Web Vitals per page. Proven in the M0 spike on macOS, Linux and Windows.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::browser::{SetDownloadBehaviorBehavior, SetDownloadBehaviorParams};
use chromiumoxide::cdp::browser_protocol::fetch::{
    ContinueRequestParams, EnableParams as FetchEnableParams, EventRequestPaused, FailRequestParams,
};
use chromiumoxide::cdp::browser_protocol::network::ErrorReason;
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::page::Page;
use futures::StreamExt;
use serde_json::Value;

use super::Rendered;
use crate::rules::{AxePass, AxeViolation, PerfMetrics, RenderedData};
use crate::{Error, Options, Result};

/// Bundled unmodified (MPL-2.0); see THIRD_PARTY.md.
const AXE: &str = include_str!("../../assets/axe.min.js");
const TABS: usize = 2;
const PAGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Waits for the largest contentful paint, then 1 s more for layout shifts, 5 s at most.
/// Observers with `buffered: true` also see entries from before they were created.
const VITALS_JS: &str = r#"new Promise(function (resolve) {
  var lcp = 0, cls = 0, t0 = performance.now(), done = false;
  function finish() { if (!done) { done = true; resolve({ lcp: lcp, cls: cls }); } }
  try {
    new PerformanceObserver(function (l) { l.getEntries().forEach(function (e) { lcp = e.startTime; }); })
      .observe({ type: 'largest-contentful-paint', buffered: true });
    new PerformanceObserver(function (l) { l.getEntries().forEach(function (e) { if (!e.hadRecentInput) cls += e.value; }); })
      .observe({ type: 'layout-shift', buffered: true });
  } catch (e) {}
  (function poll() {
    var waited = performance.now() - t0;
    if ((lcp > 0 && waited >= 1000) || waited >= 5000) finish(); else setTimeout(poll, 100);
  })();
})"#;

/// Navigation and resource timing, as the TS package's Playwright provider read them.
const STATS_JS: &str = r#"(function () {
  var nav = performance.getEntriesByType('navigation')[0];
  var res = performance.getEntriesByType('resource');
  var transfer = res.reduce(function (s, r) { return s + (r.transferSize || 0); }, 0) + ((nav && nav.transferSize) || 0);
  return {
    ttfb: nav ? Math.round(nav.responseStart) : 0,
    load: nav ? Math.round(nav.duration) : 0,
    dom: document.querySelectorAll('*').length,
    requests: res.length + 1,
    transfer: transfer
  };
})()"#;

const AXE_RUN_JS: &str = r#"axe.run(document).then(function (r) {
  return {
    violations: r.violations.map(function (v) {
      return { id: v.id, impact: v.impact, help: v.help, description: v.description, helpUrl: v.helpUrl,
               nodes: v.nodes.length, sample: v.nodes.length ? v.nodes[0].html : null };
    }),
    passes: r.passes.map(function (p) { return { id: p.id, help: p.help }; })
  };
})"#;

/// The running Chrome. Dropping it kills the process (chromiumoxide sets `kill_on_drop`) and
/// removes the profile folder.
struct Running {
    browser: Browser,
    handler: tokio::task::JoinHandle<()>,
    profile: PathBuf,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.handler.abort();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

pub(crate) struct Renderer {
    running: Option<Running>,
    executable: PathBuf,
    user_agent: String,
    /// Same policy as the fetcher: refuse private addresses when the start URL is public.
    block_private: bool,
    restarted: bool,
}

impl Renderer {
    pub(crate) async fn launch(options: &Options, block_private: bool) -> Result<Self> {
        let executable = find_chrome(options)?;
        let mut r =
            Self { running: None, executable, user_agent: options.user_agent(), block_private, restarted: false };
        r.start().await?;
        Ok(r)
    }

    async fn start(&mut self) -> Result<()> {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let profile = std::env::temp_dir().join(format!("sitevet-chrome-{}-{n}", std::process::id()));
        let mut config = BrowserConfig::builder()
            .chrome_executable(&self.executable)
            .new_headless_mode()
            .user_data_dir(&profile)
            .window_size(1280, 800)
            .viewport(None)
            .launch_timeout(Duration::from_secs(30));
        if running_as_root() {
            tracing::warn!("running as root: Chrome's sandbox is off");
            config = config.no_sandbox();
        }
        let config = config.build().map_err(Error::BrowserFailed)?;
        let (browser, mut handler) = Browser::launch(config).await.map_err(|e| Error::BrowserFailed(e.to_string()))?;
        let handler = tokio::spawn(async move { while handler.next().await.is_some() {} });
        // No downloads, ever.
        let _ = browser.execute(SetDownloadBehaviorParams::new(SetDownloadBehaviorBehavior::Deny)).await;
        self.running = Some(Running { browser, handler, profile });
        Ok(())
    }

    pub(crate) async fn render(&mut self, urls: &[String]) -> Rendered {
        let mut pages = HashMap::new();
        let mut failed = None;
        let mut queue: Vec<&String> = urls.iter().collect();
        while !queue.is_empty() {
            let Some(running) = &self.running else { break };
            let (browser, ua, guard) = (&running.browser, self.user_agent.as_str(), self.block_private);
            // Futures built up front: a closure inside the stream trips a compiler limit on Send.
            let jobs: Vec<_> = queue.iter().map(|url| render_owned(browser, (*url).clone(), ua, guard)).collect();
            let batch: Vec<(String, RenderedData)> = futures::stream::iter(jobs).buffer_unordered(TABS).collect().await;
            let alive = running.browser.version().await.is_ok();
            if alive {
                pages.extend(batch);
                break;
            }
            // Chrome died. Keep the pages that rendered, restart once for the rest.
            let done: Vec<(String, RenderedData)> = batch.into_iter().filter(|(_, d)| d.ok).collect();
            queue.retain(|u| !done.iter().any(|(d, _)| d == *u));
            pages.extend(done);
            self.running = None;
            if self.restarted {
                failed = Some("the browser crashed twice".to_string());
                break;
            }
            self.restarted = true;
            tracing::warn!("the browser crashed, restarting it once");
            if let Err(e) = self.start().await {
                failed = Some(e.to_string());
                break;
            }
        }
        Rendered { pages, failed }
    }

    pub(crate) async fn rendered_html(&self, url: &str) -> Option<String> {
        let running = self.running.as_ref()?;
        let html = async {
            let (page, guard) = open(&running.browser, url, &self.user_agent, self.block_private).await.ok()?;
            let v = eval(&page, "document.documentElement.outerHTML").await.ok();
            let _ = page.close().await;
            drop(guard);
            v?.as_str().map(str::to_string)
        };
        tokio::time::timeout(PAGE_TIMEOUT, html).await.ok().flatten()
    }

    pub(crate) async fn close(mut self) {
        if let Some(mut running) = self.running.take() {
            // A hard kill: the profile is throwaway, and a polite close took over 1 s in M0.
            let _ = running.browser.kill().await;
        }
    }
}

/// Aborts a tab's request-guard task when the tab is done.
struct GuardTask(Option<tokio::task::JoinHandle<()>>);

impl Drop for GuardTask {
    fn drop(&mut self) {
        if let Some(t) = self.0.take() {
            t.abort();
        }
    }
}

async fn open(
    browser: &Browser,
    url: &str,
    user_agent: &str,
    block_private: bool,
) -> chromiumoxide::Result<(Page, GuardTask)> {
    // Open blank, bring to front, then navigate: a background tab records no layout shifts (M0).
    let page = browser.new_page("about:blank").await?;
    let _ = page.bring_to_front().await;
    let _ = page.set_user_agent(user_agent).await;
    let guard = if block_private { GuardTask(Some(guard_requests(&page).await?)) } else { GuardTask(None) };
    page.goto(url).await?;
    Ok((page, guard))
}

/// Holds every request the tab makes (navigations, redirects, scripts, images) until its host
/// is checked: private, loopback, link-local and metadata addresses fail, the rest continue.
/// The same rule the fetcher applies, so a public page cannot make Chrome reach a private
/// address. Chrome resolves the name again after the check, so DNS rebinding in between is
/// not covered here; the fetcher's own requests are.
async fn guard_requests(page: &Page) -> chromiumoxide::Result<tokio::task::JoinHandle<()>> {
    let mut paused = page.event_listener::<EventRequestPaused>().await?;
    page.execute(FetchEnableParams::default()).await?;
    let page = page.clone();
    Ok(tokio::spawn(async move {
        while let Some(event) = paused.next().await {
            let id = event.request_id.clone();
            if request_allowed(&event.request.url).await {
                let _ = page.execute(ContinueRequestParams::new(id)).await;
            } else {
                tracing::debug!("blocked a browser request to a private address");
                let _ = page.execute(FailRequestParams::new(id, ErrorReason::BlockedByClient)).await;
            }
        }
    }))
}

async fn request_allowed(raw: &str) -> bool {
    match url::Url::parse(raw) {
        Ok(u) if matches!(u.scheme(), "http" | "https" | "ws" | "wss") => !crate::crawl::host_is_private(&u).await,
        // data:, blob: and about: never leave the browser.
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn render_owned(browser: &Browser, url: String, user_agent: &str, guard: bool) -> (String, RenderedData) {
    let data = render_page(browser, &url, user_agent, guard).await;
    (url, data)
}

async fn render_page(browser: &Browser, url: &str, user_agent: &str, block_private: bool) -> RenderedData {
    let work = async {
        let (page, guard) = open(browser, url, user_agent, block_private).await.map_err(|e| e.to_string())?;
        let result = measure(&page).await;
        let _ = page.close().await;
        drop(guard);
        result
    };
    match tokio::time::timeout(PAGE_TIMEOUT, work).await {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => RenderedData { ok: false, error: Some(e), ..Default::default() },
        Err(_) => RenderedData {
            ok: false,
            error: Some("the page did not finish rendering in 30 s".into()),
            ..Default::default()
        },
    }
}

async fn measure(page: &Page) -> std::result::Result<RenderedData, String> {
    let vitals = eval(page, VITALS_JS).await.map_err(|e| e.to_string())?;
    let stats = eval(page, STATS_JS).await.map_err(|e| e.to_string())?;
    let n = |v: &Value| v.as_f64().unwrap_or(0.0);
    let metrics = PerfMetrics {
        ttfb_ms: n(&stats["ttfb"]),
        load_ms: n(&stats["load"]),
        lcp_ms: n(&vitals["lcp"]).round(),
        cls: (n(&vitals["cls"]) * 1000.0).round() / 1000.0,
        dom_nodes: n(&stats["dom"]) as u32,
        requests: n(&stats["requests"]) as u32,
        transfer_bytes: n(&stats["transfer"]) as u64,
    };
    // axe can fail on unusual pages; the metrics still count.
    let (axe, axe_passes) = run_axe(page).await.unwrap_or_default();
    Ok(RenderedData { ok: true, error: None, axe, axe_passes, metrics: Some(metrics) })
}

async fn run_axe(page: &Page) -> Option<(Vec<AxeViolation>, Vec<AxePass>)> {
    eval_unit(page, AXE).await.ok()?;
    let r = eval(page, AXE_RUN_JS).await.ok()?;
    let s = |v: &Value| v.as_str().unwrap_or("").to_string();
    let violations = r["violations"]
        .as_array()?
        .iter()
        .map(|v| AxeViolation {
            id: s(&v["id"]),
            impact: v["impact"].as_str().map(str::to_string),
            help: s(&v["help"]),
            description: s(&v["description"]),
            help_url: s(&v["helpUrl"]),
            nodes: v["nodes"].as_u64().unwrap_or(0) as u32,
            sample: v["sample"].as_str().map(str::to_string),
        })
        .collect();
    let passes = r["passes"].as_array()?.iter().map(|p| AxePass { id: s(&p["id"]), help: s(&p["help"]) }).collect();
    Some((violations, passes))
}

fn params(js: &str, by_value: bool) -> chromiumoxide::Result<EvaluateParams> {
    EvaluateParams::builder()
        .expression(js)
        .await_promise(true)
        .return_by_value(by_value)
        .build()
        .map_err(chromiumoxide::error::CdpError::msg)
}

/// Evaluates our own script in the page and returns its JSON value.
async fn eval(page: &Page, js: &str) -> chromiumoxide::Result<Value> {
    let r = page.evaluate_expression(params(js, true)?).await?;
    Ok(r.into_value::<Value>().unwrap_or(Value::Null))
}

/// Evaluates our own script for its side effects (defining `axe`).
async fn eval_unit(page: &Page, js: &str) -> chromiumoxide::Result<()> {
    page.evaluate_expression(params(js, false)?).await.map(|_| ())
}

/// Linux CI containers often run as root, where Chrome refuses to start sandboxed.
fn running_as_root() -> bool {
    cfg!(target_os = "linux")
        && std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| s.lines().find(|l| l.starts_with("Uid:")).map(|l| l.split_whitespace().nth(1) == Some("0")))
            .unwrap_or(false)
}

/// `chrome_path`, then `SITEVET_CHROME`, then the usual install places for this OS.
fn find_chrome(options: &Options) -> Result<PathBuf> {
    if let Some(p) = &options.chrome_path {
        return if p.is_file() {
            Ok(p.clone())
        } else {
            Err(Error::BrowserNotFound { tried: vec![format!("{} (chrome_path)", p.display())] })
        };
    }
    if let Ok(p) = std::env::var("SITEVET_CHROME") {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(Error::BrowserNotFound { tried: vec![format!("{} (SITEVET_CHROME)", p.display())] })
        };
    }
    let candidates = candidates();
    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .ok_or_else(|| Error::BrowserNotFound { tried: candidates.iter().map(|p| p.display().to_string()).collect() })
}

fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let names: &[&str] = if cfg!(windows) {
        &["chrome.exe"]
    } else {
        &["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome"]
    };
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            out.extend(names.iter().map(|n| dir.join(n)));
        }
    }
    if cfg!(target_os = "macos") {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        for base in [Path::new("/Applications").to_path_buf(), home.join("Applications")] {
            out.push(base.join("Google Chrome.app/Contents/MacOS/Google Chrome"));
            out.push(base.join("Chromium.app/Contents/MacOS/Chromium"));
        }
    }
    if cfg!(windows) {
        for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(base) = std::env::var_os(var).map(PathBuf::from) {
                out.push(base.join(r"Google\Chrome\Application\chrome.exe"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::Renderer;

    /// Counts connections to a local port.
    fn listener() -> (u16, Arc<AtomicUsize>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        std::thread::spawn(move || {
            for _ in l.incoming().flatten() {
                h.fetch_add(1, Ordering::SeqCst);
            }
        });
        (port, hits)
    }

    /// A page Chrome loads from a public-looking start must not reach private addresses.
    #[tokio::test]
    async fn chrome_requests_to_private_addresses_are_blocked() {
        if std::env::var("SITEVET_TEST_CHROME").as_deref() != Ok("1") {
            return;
        }
        let (port, hits) = listener();
        let page = format!(
            "data:text/html,<title>x</title><img src=\"http://127.0.0.1:{port}/img\"><script>fetch('http://127.0.0.1:{port}/api').catch(()=>{{}})</script>"
        );
        let mut options = crate::Options::default();
        options.browser = true;

        let mut open = Renderer::launch(&options, false).await.unwrap();
        let out = open.render(std::slice::from_ref(&page)).await;
        open.close().await;
        assert!(out.pages[&page].ok);
        assert!(hits.load(Ordering::SeqCst) > 0, "without the guard Chrome reaches the listener");

        hits.store(0, Ordering::SeqCst);
        let mut guarded = Renderer::launch(&options, true).await.unwrap();
        let out = guarded.render(std::slice::from_ref(&page)).await;
        guarded.close().await;
        assert!(out.pages[&page].ok, "{:?}", out.pages[&page].error);
        assert_eq!(hits.load(Ordering::SeqCst), 0, "with the guard nothing reaches the private address");
    }
}
