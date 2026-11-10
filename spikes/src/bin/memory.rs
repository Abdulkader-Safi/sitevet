//! M0 memory spike: parse N generated pages with scraper inside spawn_blocking,
//! keep a PageData-like struct per page, drop the DOM, report peak RSS.
//! Usage: memory [pages=1000]. Exits 1 if projected 1,000-page peak >= 500 MB.

use std::sync::Arc;
use std::time::Instant;

use futures::StreamExt;
use scraper::{Html, Selector};
use sitevet_spikes::peak_rss_mb;

#[allow(dead_code)] // fields exist to hold memory like the real PageData will
struct Link {
    href: String,
    text: String,
    rel: Option<String>,
}
#[allow(dead_code)]
struct Img {
    src: String,
    alt: Option<String>,
    width: Option<String>,
    height: Option<String>,
}
#[allow(dead_code)]
struct PageData {
    url: String,
    title: Option<String>,
    meta: Vec<(String, String)>,
    headings: Vec<(u8, String)>,
    links: Vec<Link>,
    images: Vec<Img>,
    scripts: Vec<String>,
    json_ld: Vec<String>,
    text: String,
    word_count: usize,
}

impl PageData {
    fn heap_bytes(&self) -> usize {
        let s = |x: &String| x.capacity();
        let o = |x: &Option<String>| x.as_ref().map_or(0, |v| v.capacity());
        self.url.capacity()
            + o(&self.title)
            + self.meta.iter().map(|(a, b)| s(a) + s(b)).sum::<usize>()
            + self.headings.iter().map(|(_, t)| s(t)).sum::<usize>()
            + self.links.iter().map(|l| s(&l.href) + s(&l.text) + o(&l.rel)).sum::<usize>()
            + self.images.iter().map(|i| s(&i.src) + o(&i.alt) + o(&i.width) + o(&i.height)).sum::<usize>()
            + self.scripts.iter().map(s).sum::<usize>()
            + self.json_ld.iter().map(s).sum::<usize>()
            + self.text.capacity()
            + self.links.capacity() * size_of::<Link>()
            + self.images.capacity() * size_of::<Img>()
    }
}

struct Sel {
    title: Selector,
    meta: Selector,
    headings: Selector,
    links: Selector,
    images: Selector,
    scripts: Selector,
    json_ld: Selector,
    body: Selector,
}

fn extract(url: String, html: &str, sel: &Sel) -> PageData {
    let doc = Html::parse_document(html);
    let attr = |e: &scraper::ElementRef, a: &str| e.value().attr(a).map(str::to_string);
    let text_of = |e: scraper::ElementRef| e.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
    let text = doc.select(&sel.body).next().map(text_of).unwrap_or_default();
    let word_count = text.split_whitespace().count();
    PageData {
        url,
        title: doc.select(&sel.title).next().map(text_of),
        meta: doc
            .select(&sel.meta)
            .filter_map(|m| Some((attr(&m, "name").or(attr(&m, "property"))?, attr(&m, "content")?)))
            .collect(),
        headings: doc.select(&sel.headings).map(|h| (h.value().name().as_bytes()[1] - b'0', text_of(h))).collect(),
        links: doc
            .select(&sel.links)
            .map(|a| Link { href: attr(&a, "href").unwrap_or_default(), text: text_of(a), rel: attr(&a, "rel") })
            .collect(),
        images: doc
            .select(&sel.images)
            .map(|i| Img { src: attr(&i, "src").unwrap_or_default(), alt: attr(&i, "alt"), width: attr(&i, "width"), height: attr(&i, "height") })
            .collect(),
        scripts: doc.select(&sel.scripts).filter_map(|s| attr(&s, "src")).collect(),
        json_ld: doc.select(&sel.json_ld).map(|s| s.text().collect()).collect(),
        text,
        word_count,
    }
    // `doc` (the non-Send DOM) is dropped here, inside the blocking task.
}

const WORDS: &[&str] = &[
    "audit", "page", "search", "content", "rust", "crawl", "link", "image", "title", "score", "report", "fast",
    "the", "and", "of", "to", "in", "is", "for", "with", "site", "user", "data", "engine", "local", "service",
];

/// Deterministic page of roughly `target` bytes, shaped like a CMS page.
fn gen_page(i: usize, target: usize) -> String {
    let mut seed = (i as u64).wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as usize
    };
    let mut h = String::with_capacity(target + 4096);
    h.push_str(&format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Page {i} about audits</title>\
         <meta name=\"description\" content=\"Description for page {i}\"><meta property=\"og:title\" content=\"Page {i}\">\
         <link rel=\"stylesheet\" href=\"/css/site.css\"><script src=\"/js/app.js\"></script>\
         <script type=\"application/ld+json\">{{\"@context\":\"https://schema.org\",\"@type\":\"Article\",\"headline\":\"Page {i}\"}}</script>\
         <style>{}</style></head><body><nav>",
        ".c{color:#333}".repeat(200)
    ));
    for n in 0..60 {
        h.push_str(&format!("<a href=\"/nav/{n}\" class=\"nav-link\">Section {n}</a>"));
    }
    h.push_str("</nav><main>");
    while h.len() < target {
        match next() % 10 {
            0 => h.push_str(&format!("<h{lvl}>Heading {}</h{lvl}>", next() % 1000, lvl = 2 + next() % 3)),
            1 => h.push_str(&format!(
                "<img src=\"/img/{}.jpg\" alt=\"photo {}\" width=\"800\" height=\"600\" loading=\"lazy\">",
                next() % 5000,
                next() % 100
            )),
            2 => h.push_str(&format!("<p>See <a href=\"/p/{}\" rel=\"nofollow\">related page</a>.</p>", next() % 10000)),
            3 => h.push_str("<div class=\"wrap\"><div class=\"inner\"><span class=\"x\"></span></div></div>"),
            _ => {
                h.push_str("<p>");
                for _ in 0..60 {
                    h.push_str(WORDS[next() % WORDS.len()]);
                    h.push(' ');
                }
                h.push_str("</p>");
            }
        }
    }
    h.push_str("</main><footer>footer text</footer><script>window.x=1;</script></body></html>");
    h
}

/// Page size mix: 90% about 100 KB, 9% about 500 KB, 1% about 3 MB, plus page 0 at the 10 MB cap.
fn size_for(i: usize) -> usize {
    match i {
        0 => 10 * 1024 * 1024,
        _ if i % 100 == 1 => 3 * 1024 * 1024,
        _ if i % 11 == 0 => 500 * 1024,
        _ => 100 * 1024,
    }
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(1000);
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let base = peak_rss_mb();
    let sel = Arc::new(Sel {
        title: Selector::parse("title").unwrap(),
        meta: Selector::parse("meta").unwrap(),
        headings: Selector::parse("h1,h2,h3,h4,h5,h6").unwrap(),
        links: Selector::parse("a[href]").unwrap(),
        images: Selector::parse("img").unwrap(),
        scripts: Selector::parse("script[src]").unwrap(),
        json_ld: Selector::parse("script[type=\"application/ld+json\"]").unwrap(),
        body: Selector::parse("body").unwrap(),
    });
    let t = Instant::now();
    let mut html_bytes = 0usize;
    let pages: Vec<PageData> = rt.block_on(async {
        futures::stream::iter(0..n)
            .map(|i| {
                let sel = sel.clone();
                // The page arrives as a String, like a fetched body, and is dropped after parsing.
                let html = gen_page(i, size_for(i));
                let len = html.len();
                async move {
                    let pd = tokio::task::spawn_blocking(move || extract(format!("https://example.test/p/{i}"), &html, &sel))
                        .await
                        .unwrap();
                    (len, pd)
                }
            })
            .buffer_unordered(5)
            .map(|(len, pd)| {
                html_bytes += len;
                pd
            })
            .collect()
            .await
    });
    let secs = t.elapsed().as_secs_f64();
    let kept: usize = pages.iter().map(PageData::heap_bytes).sum();
    let words: usize = pages.iter().map(|p| p.word_count).sum();
    let links: usize = pages.iter().map(|p| p.links.len()).sum();
    let peak = peak_rss_mb();
    println!("pages parsed:        {}", pages.len());
    println!("html parsed:         {:.0} MB", html_bytes as f64 / 1_048_576.0);
    println!("time:                {secs:.1} s ({:.1} ms per page, 5 in parallel)", secs * 1000.0 / n as f64);
    println!("words, links kept:   {words}, {links}");
    println!("PageData kept:       {:.1} MB ({:.1} KB per page)", kept as f64 / 1_048_576.0, kept as f64 / 1024.0 / n as f64);
    println!("RSS before:          {base:.1?} MB");
    println!("peak RSS:            {peak:.1?} MB");
    if let Some(peak) = peak {
        // Peak = fixed cost (runtime, parse buffers for 5 pages in flight) + PageData per page.
        let per_page_mb = kept as f64 / 1_048_576.0 / n as f64;
        let p1000 = peak + per_page_mb * (1000usize.saturating_sub(n)) as f64;
        let p10000 = peak + per_page_mb * (10_000usize.saturating_sub(n)) as f64;
        println!("projected 1,000:     {p1000:.0} MB (limit 500)");
        println!("projected 10,000:    {p10000:.0} MB");
        println!("[{}] memory on {}", if p1000 < 500.0 { "PASS" } else { "FAIL" }, std::env::consts::OS);
        std::process::exit(if p1000 < 500.0 { 0 } else { 1 });
    }
    println!("[SKIP] peak RSS not available on {}", std::env::consts::OS);
}
