//! Runs an audit with `sitevet::spawn` from `futures::executor`, with no tokio runtime of your
//! own. This is how an app on another executor (GPUI, smol) drives sitevet.
//!
//! cargo run -p sitevet --example spawn-futures -- https://example.com

use futures::executor::block_on;
use sitevet::{AuditEvent, Options, spawn};

fn main() {
    let Some(url) = std::env::args().nth(1) else {
        eprintln!("usage: spawn-futures <url>");
        std::process::exit(2);
    };
    let mut options = Options::default();
    options.max_pages = 10;

    let handle = spawn(&url, options);
    let events = handle.events();
    // Read events on one thread while another waits for the report.
    let printer = std::thread::spawn(move || {
        block_on(async {
            while let Ok(event) = events.recv().await {
                if let AuditEvent::PageFetched { index, url, status, .. } = event {
                    eprintln!("{index:>3}  {status}  {url}");
                }
            }
        })
    });
    match block_on(handle.report()) {
        Ok(report) => println!("{} pages, score {}", report.pages.len(), report.score),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
    let _ = printer.join();
}
