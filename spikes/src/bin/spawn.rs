//! M0 spawn spike: sitevet owns a 2-thread tokio runtime, the caller drives the
//! handle from `futures::executor::block_on` with no tokio context.
//! Cases: full run, cancel mid-crawl, drop the handle mid-crawl. Exits 1 on failure.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use futures::StreamExt;
use sitevet_spikes::{Resp, serve};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
enum Event {
    Start { pages: usize },
    PageFetched { url: String, status: u16, bytes: usize },
    Complete { fetched: usize, cancelled: bool },
}

#[derive(Debug)]
struct Report {
    fetched: usize,
    cancelled: bool,
}

/// The shape `sitevet::spawn` will have: events, cancel, report, from any executor.
struct AuditHandle {
    events: async_channel::Receiver<Event>,
    cancel: CancellationToken,
    report: Option<tokio::sync::oneshot::Receiver<Report>>,
    rt: Option<tokio::runtime::Runtime>,
}

impl AuditHandle {
    fn cancel(&self) {
        self.cancel.cancel();
    }
    async fn report(&mut self) -> Option<Report> {
        self.report.take()?.await.ok()
    }
}

impl Drop for AuditHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
        // Never blocks and never panics, even when dropped inside another runtime.
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}

fn spawn(urls: Vec<String>) -> AuditHandle {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .thread_name("sitevet-spike")
        .build()
        .expect("runtime");
    let (tx, rx) = async_channel::bounded(64);
    let (report_tx, report_rx) = tokio::sync::oneshot::channel();
    let cancel = CancellationToken::new();
    let token = cancel.clone();

    rt.spawn(async move {
        let client = reqwest::Client::builder()
            .user_agent("sitevet/0.0.0-spike")
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap();
        let _ = tx.send(Event::Start { pages: urls.len() }).await;
        let fetched = AtomicUsize::new(0);
        let crawl = futures::stream::iter(urls)
            .map(|url| {
                let client = &client;
                async move {
                    let resp = client.get(&url).send().await.ok()?;
                    let status = resp.status().as_u16();
                    let bytes = resp.bytes().await.ok()?.len();
                    Some(Event::PageFetched { url, status, bytes })
                }
            })
            .buffer_unordered(5)
            .for_each(|ev| {
                let tx = &tx;
                let fetched = &fetched;
                async move {
                    if let Some(ev) = ev {
                        fetched.fetch_add(1, Ordering::SeqCst);
                        let _ = tx.send(ev).await;
                    }
                }
            });
        // Dropping `crawl` on cancel drops every in-flight request and its connection.
        let cancelled = tokio::select! {
            _ = token.cancelled() => true,
            _ = crawl => false,
        };
        let fetched = fetched.load(Ordering::SeqCst);
        let _ = tx.send(Event::Complete { fetched, cancelled }).await;
        let _ = report_tx.send(Report { fetched, cancelled });
    });

    AuditHandle { events: rx, cancel, report: Some(report_rx), rt: Some(rt) }
}

static PANICS: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        PANICS.fetch_add(1, Ordering::SeqCst);
        default_hook(info);
    }));

    // /fast/N answers after 20 ms. /slow/N drips for 2 s, so a crawl is mid-flight when cancelled.
    let server = serve(|path| {
        if path.starts_with("/fast/") {
            Resp::Delayed(Duration::from_millis(20), format!("<html><body>{path}</body></html>"))
        } else if path.starts_with("/slow/") {
            Resp::Drip { chunks: 40, every: Duration::from_millis(50) }
        } else {
            Resp::NotFound
        }
    });
    let urls = |kind: &str| (0..100).map(|i| server.url(&format!("/{kind}/{i}"))).collect::<Vec<_>>();
    let mut failed = 0;
    let mut check = |name: &str, ok: bool, detail: String| {
        println!("[{}] {name}: {detail}", if ok { "PASS" } else { "FAIL" });
        failed += usize::from(!ok);
    };

    // 1. Full run: every event, then the report.
    let t = Instant::now();
    let (events, report) = futures::executor::block_on(async {
        let mut h = spawn(urls("fast"));
        let mut events = Vec::new();
        while let Ok(ev) = h.events.recv().await {
            events.push(ev);
        }
        (events, h.report().await)
    });
    let pages = events.iter().filter(|e| matches!(e, Event::PageFetched { .. })).count();
    let shape_ok = matches!(events.first(), Some(Event::Start { pages: 100 }))
        && matches!(events.last(), Some(Event::Complete { fetched: 100, cancelled: false }));
    check(
        "full run",
        pages == 100 && shape_ok && report.as_ref().is_some_and(|r| r.fetched == 100 && !r.cancelled),
        format!("{} events, {pages} pages, report {report:?}, {} ms", events.len(), t.elapsed().as_millis()),
    );
    if let Some(Event::PageFetched { url, status, bytes }) = events.get(1) {
        println!("       sample event: {url} {status} {bytes} B");
    }

    // 2. Cancel mid-crawl: report must come back within 1 s, connections closed.
    let (ms, report, after) = futures::executor::block_on(async {
        let mut h = spawn(urls("slow"));
        // Wait until requests are in flight.
        while server.open_connections() < 5 {
            futures_timer_lite(Duration::from_millis(10)).await;
        }
        let t = Instant::now();
        h.cancel();
        let report = h.report().await;
        let ms = t.elapsed().as_millis();
        drop(h);
        futures_timer_lite(Duration::from_millis(300)).await;
        (ms, report, server.open_connections())
    });
    check(
        "cancel",
        ms < 1000 && report.as_ref().is_some_and(|r| r.cancelled) && after == 0,
        format!("report in {ms} ms: {report:?}, open connections after 300 ms: {after}"),
    );

    // 3. Drop the handle mid-crawl from a non-tokio executor: no panic, connections closed.
    let (ms, after) = futures::executor::block_on(async {
        let h = spawn(urls("slow"));
        while server.open_connections() < 5 {
            futures_timer_lite(Duration::from_millis(10)).await;
        }
        let t = Instant::now();
        drop(h);
        let ms = t.elapsed().as_millis();
        futures_timer_lite(Duration::from_millis(300)).await;
        (ms, server.open_connections())
    });
    check("drop", after == 0, format!("drop took {ms} ms, open connections after 300 ms: {after}"));

    let panics = PANICS.load(Ordering::SeqCst);
    check("no panics", panics == 0, panics.to_string());
    println!("spawn spike on {}: {failed} failed", std::env::consts::OS);
    std::process::exit(if failed == 0 { 0 } else { 1 });
}

/// A sleep that needs no runtime: a thread wakes the waker. Spike-only.
fn futures_timer_lite(d: Duration) -> impl std::future::Future<Output = ()> {
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    std::thread::spawn(move || {
        std::thread::sleep(d);
        let _ = tx.send(());
    });
    async move {
        let _ = rx.await;
    }
}
