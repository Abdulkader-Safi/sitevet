//! M0 browser spike: chromiumoxide + installed Chrome, axe-core, LCP/CLS,
//! screenshots, and cancel with a process check. Exits 1 if any check fails.
//!
//! Needs `spikes/assets/axe.min.js` (run `spikes/fetch-axe.sh`).
//! Chrome path: `SITEVET_CHROME`, else chromiumoxide's own detection.

use std::time::{Duration, Instant};

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::page::{Page, ScreenshotParams};
use futures::StreamExt;
use serde_json::Value;
use sitevet_spikes::{Resp, count_processes, serve};
use tokio_util::sync::CancellationToken;

const TEST_PAGE: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Spike page</title>
<style>body{font:16px sans-serif;margin:0} .low{color:#bbb;background:#fff}</style></head>
<body>
<main>
<h1 id="hero" style="font-size:64px;height:400px;margin:0">Largest paint candidate</h1>
<div id="slot"></div>
<p class="low">Low contrast text that axe should flag.</p>
<form><input type="text" id="noLabel"></form>
<img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=" width="40" height="40">
<div style="height:3000px;background:linear-gradient(#fff,#ccd)">tall block for full-page screenshot</div>
</main>
<script>
addEventListener('load', function () { setTimeout(function () {
  var d = document.createElement('div');
  d.style.height = '200px';
  d.textContent = 'late banner that shifts the layout';
  document.getElementById('slot').appendChild(d);
}, 400); });
</script>
</body></html>"#;

const VITALS_JS: &str = r#"new Promise(function (resolve) {
  // Wait for values instead of sleeping: slow CI VMs paint late. 5 s cap.
  var lcp = null, cls = 0, t0 = performance.now(), done = false;
  function finish() { if (!done) { done = true; resolve({ lcp: lcp, cls: cls, waited_ms: Math.round(performance.now() - t0), slot: document.getElementById('slot').offsetHeight, vis: document.visibilityState }); } }
  new PerformanceObserver(function (l) { l.getEntries().forEach(function (e) { lcp = e.startTime; }); if (lcp !== null && cls > 0) finish(); })
    .observe({ type: 'largest-contentful-paint', buffered: true });
  new PerformanceObserver(function (l) { l.getEntries().forEach(function (e) { if (!e.hadRecentInput) cls += e.value; }); if (lcp !== null && cls > 0) finish(); })
    .observe({ type: 'layout-shift', buffered: true });
  setTimeout(finish, 5000);
})"#;

const AXE_RUN_JS: &str = r#"axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa'] } })
  .then(function (r) { return { version: axe.version, violations: r.violations.map(function (v) { return { id: v.id, impact: v.impact, nodes: v.nodes.length }; }) }; })"#;

struct Checks(Vec<(String, bool, String)>);
impl Checks {
    fn add(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        let detail = detail.into();
        println!("[{}] {name}: {detail}", if ok { "PASS" } else { "FAIL" });
        self.0.push((name.into(), ok, detail));
    }
}

async fn eval(page: &Page, js: &str, by_value: bool) -> anyhow_lite::Result<Value> {
    let p = EvaluateParams::builder()
        .expression(js)
        .await_promise(true)
        .return_by_value(by_value)
        .build()
        .map_err(anyhow_lite::msg)?;
    let r = page.evaluate_expression(p).await?;
    Ok(if by_value { r.into_value::<Value>().unwrap_or(Value::Null) } else { Value::Null })
}

fn png_size(png: &[u8]) -> (u32, u32) {
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

#[tokio::main]
async fn main() {
    let mut checks = Checks(Vec::new());
    let started = Instant::now();
    let server = serve(|path| match path {
        "/" => Resp::Html(TEST_PAGE.into()),
        "/slow" => Resp::Drip { chunks: 300, every: Duration::from_millis(100) },
        _ => Resp::NotFound,
    });

    let axe = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/axe.min.js"))
        .expect("assets/axe.min.js missing: run spikes/fetch-axe.sh");

    // A unique profile dir doubles as the marker for the process check.
    let marker = format!("sitevet-spike-{}-{}", std::process::id(), started.elapsed().as_nanos());
    let profile = std::env::temp_dir().join(&marker);
    let mut cfg = BrowserConfig::builder()
        .new_headless_mode()
        .user_data_dir(&profile)
        .window_size(1280, 800)
        .viewport(None)
        .launch_timeout(Duration::from_secs(30));
    if let Ok(p) = std::env::var("SITEVET_CHROME") {
        cfg = cfg.chrome_executable(p);
    }
    let t = Instant::now();
    let (mut browser, mut handler) = match Browser::launch(cfg.build().unwrap()).await {
        Ok(b) => b,
        Err(e) => {
            checks.add("launch", false, format!("{e}"));
            std::process::exit(1);
        }
    };
    let handler_task = tokio::spawn(async move { while handler.next().await.is_some() {} });
    let version = browser.version().await.map(|v| v.product).unwrap_or_default();
    checks.add("launch", true, format!("{version} in {} ms", t.elapsed().as_millis()));

    // Load the test page and let the late banner shift the layout.
    let page = browser.new_page("about:blank").await.expect("new_page");
    page.bring_to_front().await.ok();
    page.goto(server.url("/")).await.expect("goto");
    
    // Core Web Vitals through PerformanceObserver.
    match eval(&page, VITALS_JS, true).await {
        Ok(v) => {
            let lcp = v["lcp"].as_f64();
            let cls = v["cls"].as_f64().unwrap_or(0.0);
            checks.add("lcp", lcp.is_some_and(|x| x > 0.0), format!("{lcp:?} ms"));
            checks.add("cls", cls > 0.0, format!("{cls:.4} {}", v));
        }
        Err(e) => checks.add("vitals", false, format!("{e}")),
    }

    // axe-core: inject the unmodified file, then run it.
    let t = Instant::now();
    let injected = eval(&page, &axe, false).await;
    match injected.and(eval(&page, AXE_RUN_JS, true).await) {
        Ok(v) => {
            let ids: Vec<String> = v["violations"]
                .as_array()
                .map(|a| a.iter().map(|x| x["id"].as_str().unwrap_or("").to_string()).collect())
                .unwrap_or_default();
            let ok = ids.iter().any(|i| i == "label") && ids.len() >= 3;
            checks.add(
                "axe",
                ok,
                format!("axe {} found {:?} in {} ms", v["version"], ids, t.elapsed().as_millis()),
            );
        }
        Err(e) => checks.add("axe", false, format!("{e}")),
    }

    // Screenshots: viewport and full page.
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("out");
    std::fs::create_dir_all(&out).ok();
    for (name, full) in [("viewport", false), ("fullpage", true)] {
        match page.screenshot(ScreenshotParams::builder().full_page(full).build()).await {
            Ok(png) => {
                let (w, h) = png_size(&png);
                std::fs::write(out.join(format!("{name}.png")), &png).ok();
                let ok = if full { h > 2000 } else { h <= 1000 && h > 0 };
                checks.add(&format!("screenshot {name}"), ok, format!("{w}x{h}, {} KB", png.len() / 1024));
            }
            Err(e) => checks.add(&format!("screenshot {name}"), false, format!("{e}")),
        }
    }

    // Cancel mid-load: navigate to a page that never finishes, cancel at 700 ms.
    let before = count_processes(&marker);
    let cancel = CancellationToken::new();
    let c2 = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(700)).await;
        c2.cancel();
    });
    let slow_url = server.url("/slow");
    tokio::select! {
        r = browser.new_page(slow_url) => checks.add("cancel", false, format!("slow page finished early: {:?}", r.is_ok())),
        _ = cancel.cancelled() => {
            let t = Instant::now();
            // Cancel policy: hard kill (the profile is throwaway). SPIKE_POLITE_CLOSE=1
            // tries Browser.close for up to 300 ms first, for comparison.
            let polite = std::env::var("SPIKE_POLITE_CLOSE").is_ok();
            let closed = polite && tokio::time::timeout(Duration::from_millis(300), browser.close()).await.is_ok_and(|r| r.is_ok());
            if !closed {
                browser.kill().await;
            } else {
                let _ = tokio::time::timeout(Duration::from_millis(500), browser.wait()).await;
            }
            handler_task.abort();
            let mut left = count_processes(&marker);
            while left > 0 && t.elapsed() < Duration::from_secs(3) {
                tokio::time::sleep(Duration::from_millis(25)).await;
                left = count_processes(&marker);
            }
            let ms = t.elapsed().as_millis();
            checks.add(
                "cancel",
                left == 0 && ms < 1000,
                format!("{before} Chrome processes before, {left} after {ms} ms (graceful close: {closed})"),
            );
        }
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    checks.add("server connections after cancel", server.open_connections() == 0, server.open_connections().to_string());
    drop(browser);
    let _ = std::fs::remove_dir_all(&profile);

    let failed = checks.0.iter().filter(|c| !c.1).count();
    println!("browser spike on {}: {} checks, {failed} failed", std::env::consts::OS, checks.0.len());
    std::process::exit(if failed == 0 { 0 } else { 1 });
}

// ponytail: two-line error shim so the spike skips anyhow.
mod anyhow_lite {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
    pub fn msg(s: String) -> Box<dyn std::error::Error + Send + Sync> {
        s.into()
    }
}
