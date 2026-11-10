//! Entry points and progress events.

pub(crate) mod pipeline;

use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::report::{AuditReport, Status};
use crate::{Error, Options, Result};

/// Progress of one audit, in order: `AuditStart`, a `PageFetched` per page, `CrawlComplete`,
/// then `CheckComplete` and `PageComplete` as pages finish, and `AuditComplete` with the report.
/// Events after `CrawlComplete` count against a known total.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum AuditEvent {
    AuditStart { url: String, max_pages: u32 },
    PageFetched { url: String, status: u16, depth: u32, index: u32, ms: u64 },
    CrawlComplete { pages: u32, checks_total: u32, ms: u64 },
    CheckComplete { url: String, rule_id: String, status: Status, checks_done: u32, checks_total: u32 },
    PageComplete { url: String, index: u32, pages_total: u32, score: u8, findings: u32, metrics: u32, ms: u64 },
    AuditComplete { score: u8, pages: u32, checks: u32, ms: u64, report: Box<AuditReport> },
}

/// Where the engine sends events. Bounded, so a slow reader slows the audit instead of
/// growing memory. A reader that went away is ignored.
pub(crate) struct Events(Option<async_channel::Sender<AuditEvent>>);

impl Events {
    pub(crate) async fn send(&self, event: AuditEvent) {
        if let Some(tx) = &self.0 {
            let _ = tx.send(event).await;
        }
    }
}

const EVENT_CAPACITY: usize = 1024;

/// Runs a full audit. Needs a tokio context. Dropping the future stops all work.
pub async fn audit(url: &str, options: Options) -> Result<AuditReport> {
    pipeline::run(url.to_string(), options, Events(None), CancellationToken::new()).await
}

/// Starts an audit and streams its events. Needs a tokio context. Dropping the stream cancels.
pub fn audit_stream(url: &str, options: Options) -> AuditStream {
    let (tx, rx) = async_channel::bounded(EVENT_CAPACITY);
    let (report_tx, report_rx) = oneshot::channel();
    let cancel = CancellationToken::new();
    tokio::spawn(deliver(pipeline::run(url.to_string(), options, Events(Some(tx)), cancel.clone()), report_tx));
    AuditStream { events: Some(Box::pin(rx)), report: Some(report_rx), cancel }
}

/// A running audit as a [`futures::Stream`] of [`AuditEvent`]s.
pub struct AuditStream {
    events: Option<Pin<Box<async_channel::Receiver<AuditEvent>>>>,
    report: Option<oneshot::Receiver<Result<AuditReport>>>,
    cancel: CancellationToken,
}

impl futures::Stream for AuditStream {
    type Item = AuditEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<AuditEvent>> {
        match self.events.as_mut() {
            Some(rx) => rx.as_mut().poll_next(cx),
            None => Poll::Ready(None),
        }
    }
}

impl AuditStream {
    /// Stops the audit. [`AuditStream::final_report`] then returns [`Error::Cancelled`].
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Waits for the report. Events not read yet are dropped.
    pub async fn final_report(mut self) -> Result<AuditReport> {
        self.events = None;
        wait(self.report.take()).await
    }
}

impl Drop for AuditStream {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Starts an audit on a runtime sitevet owns, so it works from any executor (GPUI, smol,
/// `futures::executor`). The runtime is created on first use with
/// [`Options::runtime_threads`] workers and shut down when the last handle drops.
pub fn spawn(url: &str, options: Options) -> AuditHandle {
    let (tx, rx) = async_channel::bounded(EVENT_CAPACITY);
    let (report_tx, report_rx) = oneshot::channel();
    let cancel = CancellationToken::new();
    let runtime = shared_runtime(options.runtime_threads);
    let job = pipeline::run(url.to_string(), options, Events(Some(tx)), cancel.clone());
    match runtime.as_ref().and_then(|r| r.0.as_ref()) {
        Some(rt) => drop(rt.spawn(deliver(job, report_tx))),
        None => {
            let _ = report_tx.send(Err(Error::InvalidOptions {
                field: "runtime_threads".into(),
                reason: "could not start the audit runtime".into(),
            }));
        }
    }
    AuditHandle { events: Some(rx), report: Some(report_rx), cancel, _runtime: runtime }
}

/// A running audit started with [`spawn`]. Dropping it cancels the audit.
pub struct AuditHandle {
    events: Option<async_channel::Receiver<AuditEvent>>,
    report: Option<oneshot::Receiver<Result<AuditReport>>>,
    cancel: CancellationToken,
    _runtime: Option<Arc<OwnedRuntime>>,
}

impl AuditHandle {
    /// The event channel. Clone it freely; every clone shares the same events.
    pub fn events(&self) -> async_channel::Receiver<AuditEvent> {
        self.events.clone().unwrap_or_else(|| async_channel::bounded(1).1)
    }

    /// Stops the audit. Network work stops at once; the report resolves to [`Error::Cancelled`].
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Waits for the report. Works on any executor. Event receivers obtained from
    /// [`AuditHandle::events`] keep receiving until the audit ends.
    pub async fn report(mut self) -> Result<AuditReport> {
        // Our own receiver goes, so an app that never reads events does not stall the audit.
        self.events = None;
        wait(self.report.take()).await
    }
}

impl Drop for AuditHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

async fn wait(report: Option<oneshot::Receiver<Result<AuditReport>>>) -> Result<AuditReport> {
    match report {
        Some(rx) => rx.await.unwrap_or(Err(Error::Cancelled)),
        None => Err(Error::Cancelled),
    }
}

/// Runs the audit and sends its result to `tx`.
async fn deliver(
    job: impl std::future::Future<Output = Result<AuditReport>>,
    tx: oneshot::Sender<Result<AuditReport>>,
) {
    let _ = tx.send(job.await);
}

/// The runtime behind [`spawn`]. Shut down without blocking when the last handle drops.
struct OwnedRuntime(Option<tokio::runtime::Runtime>);

impl Drop for OwnedRuntime {
    fn drop(&mut self) {
        if let Some(rt) = self.0.take() {
            rt.shutdown_background();
        }
    }
}

static RUNTIME: Mutex<Weak<OwnedRuntime>> = Mutex::new(Weak::new());

fn shared_runtime(threads: usize) -> Option<Arc<OwnedRuntime>> {
    let mut slot = RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(rt) = slot.upgrade() {
        return Some(rt);
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(threads.max(1))
        .thread_name("sitevet")
        .enable_all()
        .build()
        .ok()?;
    let rt = Arc::new(OwnedRuntime(Some(rt)));
    *slot = Arc::downgrade(&rt);
    Some(rt)
}
