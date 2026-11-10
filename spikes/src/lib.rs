//! Shared helpers for the M0 spikes: a tiny threaded HTTP server that counts
//! open connections, and a process counter used to prove Chrome is gone.
// ponytail: throwaway spike code, not the real fixture server (that is M1, wiremock).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub enum Resp {
    Html(String),
    /// Wait, then send the page.
    Delayed(Duration, String),
    /// Send headers, then one small chunk every `every`, `chunks` times.
    Drip { chunks: usize, every: Duration },
    NotFound,
}

pub struct Server {
    pub addr: SocketAddr,
    /// Connections currently open on the server side.
    pub open: Arc<AtomicUsize>,
    /// Requests served so far.
    pub hits: Arc<AtomicUsize>,
}

impl Server {
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
    pub fn open_connections(&self) -> usize {
        self.open.load(Ordering::SeqCst)
    }
}

pub fn serve<F>(handler: F) -> Server
where
    F: Fn(&str) -> Resp + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    let open = Arc::new(AtomicUsize::new(0));
    let hits = Arc::new(AtomicUsize::new(0));
    let handler = Arc::new(handler);
    let (o, h) = (open.clone(), hits.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (handler, o, h) = (handler.clone(), o.clone(), h.clone());
            o.fetch_add(1, Ordering::SeqCst);
            std::thread::spawn(move || {
                let _ = handle(stream, &*handler, &h);
                o.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });
    Server { addr, open, hits }
}

fn handle(mut s: TcpStream, handler: &dyn Fn(&str) -> Resp, hits: &AtomicUsize) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = s.read(&mut chunk)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let head = String::from_utf8_lossy(&buf);
    let path = head.split_whitespace().nth(1).unwrap_or("/").to_string();
    hits.fetch_add(1, Ordering::SeqCst);
    let send = |s: &mut TcpStream, code: &str, body: &str| {
        write!(
            s,
            "HTTP/1.1 {code}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    };
    match handler(&path) {
        Resp::Html(b) => send(&mut s, "200 OK", &b)?,
        Resp::Delayed(d, b) => {
            std::thread::sleep(d);
            send(&mut s, "200 OK", &b)?
        }
        Resp::NotFound => send(&mut s, "404 Not Found", "not found")?,
        Resp::Drip { chunks, every } => {
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<html><body>")?;
            for i in 0..chunks {
                std::thread::sleep(every);
                // A write error means the client went away: stop and close.
                write!(s, "<p>chunk {i}</p>\n")?;
                s.flush()?;
            }
        }
    }
    Ok(())
}

/// Number of live processes whose command line contains `marker`.
pub fn count_processes(marker: &str) -> usize {
    #[cfg(unix)]
    let out = std::process::Command::new("pgrep").args(["-f", marker]).output();
    #[cfg(windows)]
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "(Get-CimInstance Win32_Process | Where-Object {{ $_.CommandLine -like '*{marker}*' -and $_.Name -notlike 'powershell*' }}).ProcessId"
            ),
        ])
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.trim().is_empty()).count(),
        Err(_) => usize::MAX,
    }
}

/// Peak resident set size of this process in MB, where the OS reports it.
pub fn peak_rss_mb() -> Option<f64> {
    #[cfg(unix)]
    unsafe {
        let mut u: libc::rusage = std::mem::zeroed();
        libc::getrusage(libc::RUSAGE_SELF, &mut u);
        let raw = u.ru_maxrss as f64;
        // macOS reports bytes, Linux reports kilobytes.
        let bytes = if cfg!(target_os = "macos") { raw } else { raw * 1024.0 };
        Some(bytes / 1_048_576.0)
    }
    #[cfg(not(unix))]
    None
}
