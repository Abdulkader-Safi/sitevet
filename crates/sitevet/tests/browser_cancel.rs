#![cfg(all(feature = "browser", unix))]
#![allow(clippy::unwrap_used)]
//! FR-API-03, the Chromium half. Its own test binary: Chrome's profile folder carries this
//! process's ID, which is how the test counts its Chrome processes, so no other test may launch
//! Chrome in the same process. Runs only with SITEVET_TEST_CHROME=1.

mod support;

use std::time::{Duration, Instant};

use futures::StreamExt;
use sitevet::{AuditEvent, Error, Options, audit_stream};
use support::{Fixture, chain_site};

fn chrome_processes() -> usize {
    let marker = format!("sitevet-chrome-{}-", std::process::id());
    let out = std::process::Command::new("pgrep").args(["-f", &marker]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).lines().count()
}

/// FR-API-03, the Chromium half: cancelling mid-crawl leaves no Chrome process after 1 s.
#[tokio::test]
async fn cancel_kills_chrome() {
    if std::env::var("SITEVET_TEST_CHROME").as_deref() != Ok("1") {
        return;
    }
    let fx = Fixture::start(chain_site(100, Duration::from_millis(300))).await;
    let mut stream = audit_stream(&fx.url("/"), {
        let mut o = Options::default();
        o.browser = true;
        o
    });
    while let Some(e) = stream.next().await {
        if matches!(e, AuditEvent::PageFetched { index: 3, .. }) {
            break;
        }
    }
    assert!(chrome_processes() > 0, "Chrome is running during the crawl");
    let t = Instant::now();
    stream.cancel();
    assert!(matches!(stream.final_report().await, Err(Error::Cancelled)));
    while chrome_processes() > 0 {
        if t.elapsed() >= Duration::from_secs(1) {
            let ps = std::process::Command::new("ps").args(["-axo", "pid,ppid,stat,command"]).output().unwrap();
            let marker = format!("sitevet-chrome-{}-", std::process::id());
            let rows: Vec<String> = String::from_utf8_lossy(&ps.stdout)
                .lines()
                .filter(|l| l.contains(&marker))
                .map(|l| l.chars().take(160).collect())
                .collect();
            panic!("Chrome still running after 1 s:\n{}", rows.join("\n"));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
