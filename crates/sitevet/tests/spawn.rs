#![allow(clippy::unwrap_used)]
//! FR-API-06: `spawn` from `futures::executor` with no tokio context anywhere on this thread.

mod support;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use futures::executor::block_on;
use sitevet::{AuditEvent, Error, spawn};
use support::{Site, ThreadFixture, chain_site, test_options};

#[test]
fn spawn_delivers_every_event_and_the_report() {
    assert!(tokio::runtime::Handle::try_current().is_err());
    let fx = ThreadFixture::start(Site::from_dir("basic"));
    let handle = spawn(&fx.url, test_options());
    let events = handle.events();
    let collected = std::thread::spawn(move || {
        block_on(async move {
            let mut all = Vec::new();
            while let Ok(e) = events.recv().await {
                all.push(e);
            }
            all
        })
    });
    let report = block_on(handle.report()).unwrap();
    let events = collected.join().unwrap();
    assert_eq!(report.pages.len(), 5);
    assert!(matches!(events.first(), Some(AuditEvent::AuditStart { .. })));
    let Some(AuditEvent::AuditComplete { report: streamed, .. }) = events.last() else { panic!("no AuditComplete") };
    assert_eq!(streamed.pages.len(), 5);
    assert_eq!(events.iter().filter(|e| matches!(e, AuditEvent::PageComplete { .. })).count(), 5);
}

#[test]
fn spawn_cancel_and_drop_stop_within_a_second() {
    let fx = ThreadFixture::start(chain_site(100, Duration::from_millis(300)));
    let url = format!("{}/", fx.proxy_url);

    let handle = spawn(&url, test_options());
    let events = handle.events();
    block_on(async {
        while let Ok(e) = events.recv().await {
            if matches!(e, AuditEvent::PageFetched { index: 2, .. }) {
                break;
            }
        }
    });
    let t = Instant::now();
    handle.cancel();
    assert!(matches!(block_on(handle.report()), Err(Error::Cancelled)));
    assert!(t.elapsed() < Duration::from_secs(1));
    wait_closed(&fx, t);

    let handle = spawn(&url, test_options());
    std::thread::sleep(Duration::from_millis(700));
    let t = Instant::now();
    drop(handle);
    wait_closed(&fx, t);
}

fn wait_closed(fx: &ThreadFixture, since: Instant) {
    while fx.open.load(Ordering::SeqCst) > 0 {
        assert!(since.elapsed() < Duration::from_secs(1), "connections still open after 1 s");
        std::thread::sleep(Duration::from_millis(10));
    }
}
