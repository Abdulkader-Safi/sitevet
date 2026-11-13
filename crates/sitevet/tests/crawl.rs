#![allow(clippy::unwrap_used)]

mod support;

use std::time::Duration;

use sitevet::{Error, Options, audit};
use support::{Fixture, Route, Site, chain_site, test_options};

fn paths(report: &sitevet::AuditReport) -> Vec<String> {
    report.pages.iter().map(|p| url::Url::parse(&p.url).unwrap().path().to_string()).collect()
}

/// FR-CRAWL-01: 150 linked pages, defaults crawl exactly 100, none deeper than 5 clicks.
#[tokio::test]
async fn defaults_crawl_100_pages_within_depth_5() {
    // `/` links to 25 pages at depth 1, each level links to the next (25 per level up to depth 5),
    // and a chain of 24 more pages hangs below depth 5: 1 + 125 + 24 = 150 pages.
    let path = |level: usize, i: usize| if level == 0 { "/".into() } else { format!("/l{level}/{i}") };
    let mut site = Site::new();
    for level in 0..=5 {
        let count = if level == 0 { 1 } else { 25 };
        for i in 0..count {
            let links: String = if level < 5 {
                (0..25).map(|j| format!("<a href=\"{}\">x</a>", path(level + 1, j))).collect()
            } else {
                "<a href=\"/deep/0\">deeper</a>".into()
            };
            site = site.page(&path(level, i), format!("<title>t</title>{links}"));
        }
    }
    for i in 0..24 {
        site = site.page(&format!("/deep/{i}"), format!("<a href=\"/deep/{}\">next</a>", i + 1));
    }
    let fx = Fixture::start(site).await;

    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    assert_eq!(report.pages.len(), 100);
    assert!(report.pages.iter().all(|p| p.depth <= 5));

    let mut wide = Options::default();
    wide.max_pages = 1000;
    let report = audit(&fx.url("/"), wide).await.unwrap();
    assert_eq!(report.pages.len(), 126, "depth 5 stops the crawl before /deep");
    assert_eq!(report.pages.iter().map(|p| p.depth).max(), Some(5));
    // The link checker may HEAD /deep/0 (it is linked), but the crawl never GETs it.
    assert!(!fx.hits().iter().any(|h| h.path.starts_with("/deep/") && h.method == "GET"));
}

/// FR-CRAWL-03: robots.txt and sitemap.xml are requested once each, even with link checks on.
/// So is llms.txt (FR-AEO-05).
#[tokio::test]
async fn robots_and_sitemap_fetched_once() {
    let robots = Route::html("User-agent: *\nAllow: /\n").content_type("text/plain");
    let sitemap = Route::html("<urlset><url><loc>{{origin}}/p3</loc></url></urlset>").content_type("application/xml");
    let llms = Route::html("# Fixture site\n\n- [Home]({{origin}}/)\n").content_type("text/plain");
    let site = chain_site(20, Duration::ZERO)
        .route("/robots.txt", robots)
        .route("/sitemap.xml", sitemap)
        .route("/llms.txt", llms);
    let fx = Fixture::start(site).await;
    let report = audit(&fx.url("/"), test_options()).await.unwrap();
    assert_eq!(report.pages.len(), 20);
    assert_eq!(fx.count("/robots.txt"), 1);
    assert_eq!(fx.count("/sitemap.xml"), 1);
    assert_eq!(fx.count("/llms.txt"), 1);
}

/// FR-CRAWL-04: a disallowed path is not requested, is listed as blocked, and is requested
/// with ignore_robots.
#[tokio::test]
async fn robots_disallow_is_obeyed_and_reported() {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let report = audit(&fx.url("/"), test_options()).await.unwrap();
    assert_eq!(fx.count("/private/secret.html"), 0, "not crawled and not link-checked");
    assert_eq!(report.blocked.len(), 1);
    assert!(report.blocked[0].url.ends_with("/private/secret.html"));
    assert_eq!(report.blocked[0].rule.as_deref(), Some("Disallow: /private"));

    let mut options = test_options();
    options.ignore_robots = true;
    let report = audit(&fx.url("/"), options).await.unwrap();
    assert!(report.blocked.is_empty());
    assert!(paths(&report).contains(&"/private/secret.html".to_string()));
}

/// FR-CRAWL-05: a page only the sitemap lists is crawled, and only when crawl_sitemap is on.
#[tokio::test]
async fn sitemap_only_page_is_crawled() {
    let fx = Fixture::start(Site::from_dir("basic")).await;
    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    let orphan = report.pages.iter().find(|p| p.url.ends_with("/orphan.html")).expect("orphan crawled");
    assert_eq!(orphan.depth, 1);

    let mut options = Options::default();
    options.crawl_sitemap = false;
    let report = audit(&fx.url("/"), options).await.unwrap();
    assert!(!paths(&report).contains(&"/orphan.html".to_string()));
}

/// FR-CRAWL-06: with delay_ms = 500, request starts to one host are at least 500 ms apart.
#[tokio::test]
async fn per_host_delay_spaces_requests() {
    let fx = Fixture::start(chain_site(4, Duration::ZERO)).await;
    let mut options = Options::default();
    options.delay_ms = 500;
    audit(&fx.url("/"), options).await.unwrap();
    let hits = fx.hits();
    assert!(hits.len() >= 6, "robots, sitemap and 4 pages: {}", hits.len());
    // The server stamps arrival, which trails the client's start by a few ms, more when a request
    // opens a new connection or the OS timer is coarse (the first request on a Windows runner has
    // arrived 55 ms late). So each gap, and the whole run, get 100 ms of slack.
    for pair in hits.windows(2) {
        let gap = pair[1].at.duration_since(pair[0].at);
        assert!(gap >= Duration::from_millis(400), "{} then {}: {gap:?}", pair[0].path, pair[1].path);
    }
    let span = hits[hits.len() - 1].at.duration_since(hits[0].at);
    assert!(span >= Duration::from_millis(500 * (hits.len() as u64 - 1) - 100), "{} requests in {span:?}", hits.len());
}

/// FR-CRAWL-07: the default user agent names sitevet, its version and the repo.
#[tokio::test]
async fn default_user_agent() {
    let fx = Fixture::start(chain_site(1, Duration::ZERO)).await;
    audit(&fx.url("/"), Options::default()).await.unwrap();
    let expected = format!("sitevet/{} (+https://github.com/Abdulkader-Safi/sitevet)", env!("CARGO_PKG_VERSION"));
    assert!(fx.hits().iter().all(|h| h.user_agent == expected));
}

/// FR-CRAWL-08: /a, /a/ and /a#x produce one audited page, and the redirect is recorded.
#[tokio::test]
async fn trailing_slash_redirect_and_fragment_dedupe() {
    let site = Site::new()
        .page("/", r#"<a href="/a">1</a><a href="/a/">2</a><a href="/a#x">3</a>"#)
        .route("/a", Route::redirect("/a/"))
        .page("/a/", "<title>A</title>");
    let fx = Fixture::start(site).await;
    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    assert_eq!(report.pages.len(), 2, "{:?}", paths(&report));
    let a = &report.pages[1];
    assert!(a.final_url.ends_with("/a/"));
    assert_eq!(a.redirect_chain.len(), 1);
}

/// FR-CRAWL-09: a URL linked from 10 pages is requested once, external or internal.
#[tokio::test]
async fn shared_link_checker_requests_each_url_once() {
    let external = Fixture::start(Site::new().page("/shared", "ok")).await;
    let shared = external.url("/shared");
    let mut site = Site::new();
    for i in 0..10 {
        let next = if i < 9 { format!("<a href=\"/p{}\">next</a>", i + 1) } else { String::new() };
        let path = if i == 0 { "/".to_string() } else { format!("/p{i}") };
        site = site.page(&path, format!("<title>{i}</title><a href=\"{shared}\">x</a><a href=\"/p9\">last</a>{next}"));
    }
    let fx = Fixture::start(site).await;
    let mut options = test_options();
    options.max_depth = 10;
    let report = audit(&fx.url("/"), options).await.unwrap();
    assert_eq!(report.pages.len(), 10);
    assert_eq!(external.count("/shared"), 1);
    assert_eq!(fx.count("/p9"), 1, "the crawl's own fetch answers the link check");
}

/// FR-CRAWL-11: an unreachable start URL is an error.
#[tokio::test]
async fn unreachable_start_is_an_error() {
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let err = audit(&format!("http://127.0.0.1:{port}/"), Options::default()).await.unwrap_err();
    assert!(matches!(err, Error::StartPageUnreachable { .. }), "{err}");

    let fx = Fixture::start(Site::new().route("/", Route::html("{}").content_type("application/json"))).await;
    let err = audit(&fx.url("/"), Options::default()).await.unwrap_err();
    assert!(matches!(err, Error::NotHtml { .. }), "{err}");

    let err = audit("ftp://example.com/", Options::default()).await.unwrap_err();
    assert!(matches!(err, Error::InvalidUrl { .. }), "{err}");
}

/// FR-CRAWL-11: one 500 page among 10 is a page with its status, and the audit goes on.
#[tokio::test]
async fn failing_page_is_reported_not_fatal() {
    let site = chain_site(10, Duration::ZERO).route("/p4", Route::html("<h1>boom</h1>").status(500));
    let fx = Fixture::start(site).await;
    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    assert_eq!(report.pages.len(), 10);
    let p4 = report.pages.iter().find(|p| p.url.ends_with("/p4")).unwrap();
    assert_eq!(p4.status, 500);
    assert!(report.pages.iter().filter(|p| p.status == 200).count() == 9);
}

/// SSRF guard: with private networks refused, a loopback start URL is never requested.
#[tokio::test]
async fn private_addresses_blocked_when_not_allowed() {
    let fx = Fixture::start(chain_site(1, Duration::ZERO)).await;
    let mut options = Options::default();
    options.allow_private_network = Some(false);
    let err = audit(&fx.url("/"), options).await.unwrap_err();
    assert!(err.to_string().contains("private network"), "{err}");
    assert!(fx.hits().is_empty());
}

/// NFR-05: HTML over 10 MB is cut and flagged.
#[tokio::test]
async fn oversized_html_is_truncated() {
    let big = format!("<title>big</title><p>{}</p>", "x".repeat(11 * 1024 * 1024));
    let fx = Fixture::start(Site::new().page("/", big)).await;
    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    assert!(report.pages[0].truncated);
    assert_eq!(report.pages[0].html_bytes, 10 * 1024 * 1024);
}

async fn gzip(data: &[u8]) -> Vec<u8> {
    use tokio::io::AsyncReadExt;
    let mut out = Vec::new();
    async_compression::tokio::bufread::GzipEncoder::new(data).read_to_end(&mut out).await.unwrap();
    out
}

/// Compressed pages decode, the compression rule sees Content-Encoding, and a decompression
/// bomb stops at the 10 MB cap.
#[tokio::test]
async fn gzip_bodies_decode_and_bombs_stop_at_the_cap() {
    let html = "<title>Compressed page</title><h1>Hello</h1>";
    let mut page = Route::html("");
    page.body = gzip(html.as_bytes()).await;
    page.headers.push(("content-encoding".into(), "gzip".into()));
    let mut bomb = Route::html("");
    bomb.body = gzip(&vec![b'a'; 20 * 1024 * 1024]).await;
    bomb.headers.push(("content-encoding".into(), "gzip".into()));
    assert!(bomb.body.len() < 100_000, "20 MB of 'a' compresses small");
    let fx = Fixture::start(Site::new().route("/", page).route("/bomb", bomb)).await;

    let report = audit(&fx.url("/"), Options::default()).await.unwrap();
    let p = &report.pages[0];
    assert!(p.findings.iter().any(|f| f.rule_id == "core-seo/title-present" && f.status == sitevet::Status::Pass));
    let compression = p.findings.iter().find(|f| f.rule_id == "performance/text-compression").unwrap();
    assert_eq!(compression.message, "Compressed (gzip)");

    let report = audit(&fx.url("/bomb"), Options::default()).await.unwrap();
    assert!(report.pages[0].truncated);
    assert_eq!(report.pages[0].html_bytes, 10 * 1024 * 1024);
}

/// A server that answers HEAD with 404 but GET with 200 has a working link.
#[tokio::test]
async fn head_error_falls_back_to_get() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let external = MockServer::start().await;
    Mock::given(method("HEAD")).and(path("/item")).respond_with(ResponseTemplate::new(404)).mount(&external).await;
    Mock::given(method("GET")).and(path("/item")).respond_with(ResponseTemplate::new(200)).mount(&external).await;
    Mock::given(path("/gone")).respond_with(ResponseTemplate::new(404)).mount(&external).await;
    let base = external.uri();
    let fx =
        Fixture::start(Site::new().page("/", format!("<a href=\"{base}/item\">a</a><a href=\"{base}/gone\">b</a>")))
            .await;
    let mut options = Options::default();
    options.only = vec!["links".into()];
    let report = audit(&fx.url("/"), options).await.unwrap();
    let broken: Vec<_> = report.pages[0]
        .findings
        .iter()
        .filter(|f| f.rule_id == "links/broken-external" && f.status != sitevet::Status::Pass)
        .map(|f| f.evidence.clone().unwrap())
        .collect();
    assert_eq!(broken, [format!("{base}/gone")]);
}

/// A windows-1252 page reads as the browser shows it, not as UTF-8 replacement characters.
#[tokio::test]
async fn legacy_charset_pages_decode() {
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    let html = "<html lang=\"fr\"><head><title>Caf\u{e9} cr\u{e8}me</title></head>\
                <body><p>Le caf\u{e9} cr\u{e8}me du caf\u{e9}.</p></body></html>";
    let latin1: Vec<u8> = html.chars().map(|c| u8::try_from(u32::from(c)).unwrap()).collect();
    Mock::given(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(latin1, "text/html; charset=windows-1252"))
        .mount(&server)
        .await;
    let mut options = Options::default();
    options.only = vec!["content".into()];
    let report = audit(&format!("{}/", server.uri()), options).await.unwrap();
    let table = report.pages[0].metrics.iter().find(|m| m.id == "content/top-words-1").unwrap();
    let sitevet::MetricBody::Table { rows, .. } = &table.body else { panic!("not a table") };
    assert_eq!(rows[0][0], sitevet::MetricCell::Text("caf\u{e9}".into()));
}
