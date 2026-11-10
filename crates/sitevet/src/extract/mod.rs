//! DOM -> PageData. Runs inside `spawn_blocking`; the DOM never leaves that call.

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

/// Everything later stages read about one page. Built from the DOM in one blocking step.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PageData {
    pub url: String,
    pub final_url: String,
    /// 0 when the fetch failed.
    pub status: u16,
    pub error: Option<String>,
    pub depth: u32,
    pub redirect_chain: Vec<String>,
    pub response_ms: u64,
    /// Lowercase names, in response order. Repeated headers stay separate.
    pub headers: Vec<(String, String)>,
    pub html_bytes: u64,
    pub truncated: bool,
    pub title: Option<String>,
    /// The `lang` attribute of `<html>`.
    pub lang: Option<String>,
    pub meta: Vec<MetaTag>,
    pub canonical: Option<String>,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub images: Vec<Image>,
    pub scripts: Vec<Script>,
    /// Absolute URLs of `<link rel="stylesheet">`.
    pub stylesheets: Vec<String>,
    /// Body text without scripts and styles, whitespace collapsed.
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MetaTag {
    /// `name`, `property` or `http-equiv`, lowercased.
    pub name: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Heading {
    pub level: u8,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Link {
    pub href: String,
    /// Absolute, fragment kept, any scheme (`mailto:` too). None when the href does not parse.
    pub url: Option<String>,
    /// Trimmed, inner whitespace kept, as the TS package reads it.
    pub text: String,
    /// Lowercased.
    pub rel: String,
    /// Same origin as the start URL.
    pub internal: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Image {
    pub src: String,
    pub url: Option<String>,
    pub alt: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub loading: Option<String>,
    pub srcset: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Script {
    /// Absolute URL, None for inline scripts.
    pub src: Option<String>,
    pub inline_bytes: u64,
    pub is_async: bool,
    pub defer: bool,
}

struct Sel {
    html: Selector,
    title: Selector,
    meta: Selector,
    canonical: Selector,
    headings: Selector,
    links: Selector,
    images: Selector,
    scripts: Selector,
    stylesheets: Selector,
    body: Selector,
}

#[allow(clippy::expect_used)] // fixed selectors, covered by every test
static SEL: LazyLock<Sel> = LazyLock::new(|| {
    let s = |css: &str| Selector::parse(css).expect("valid selector");
    Sel {
        html: s("html"),
        title: s("title"),
        meta: s("meta"),
        canonical: s(r#"link[rel~="canonical" i]"#),
        headings: s("h1,h2,h3,h4,h5,h6"),
        links: s("a[href]"),
        images: s("img"),
        scripts: s("script"),
        stylesheets: s(r#"link[rel~="stylesheet" i][href]"#),
        body: s("body"),
    }
});

/// Fills the DOM-derived fields of `page`. `page.final_url` must be set: relative URLs resolve
/// against it. `origin` is the start URL's, for the internal flag.
pub(crate) fn extract(dom: &Html, page: &mut PageData, origin: &url::Origin) {
    let sel = &*SEL;
    let base = Url::parse(&page.final_url).ok();
    let abs = |href: &str| base.as_ref().and_then(|b| b.join(href.trim()).ok());
    let attr = |e: &ElementRef, name: &str| e.value().attr(name).map(str::to_string);

    let root = dom.root_element();
    page.lang = dom.select(&sel.html).next().and_then(|h| attr(&h, "lang"));
    page.title = dom.select(&sel.title).next().map(text_of);
    page.meta = dom
        .select(&sel.meta)
        .filter_map(|m| {
            let name = attr(&m, "name").or_else(|| attr(&m, "property")).or_else(|| attr(&m, "http-equiv"))?;
            Some(MetaTag { name: name.to_lowercase(), content: attr(&m, "content").unwrap_or_default() })
        })
        .collect();
    page.canonical = dom.select(&sel.canonical).next().and_then(|l| attr(&l, "href"));
    page.headings = dom
        .select(&sel.headings)
        .map(|h| Heading { level: h.value().name().as_bytes()[1] - b'0', text: text_of(h) })
        .collect();
    page.links = dom
        .select(&sel.links)
        .map(|a| {
            let href = attr(&a, "href").unwrap_or_default();
            let url = abs(&href);
            Link {
                internal: url.as_ref().is_some_and(|u| matches!(u.scheme(), "http" | "https") && u.origin() == *origin),
                url: url.map(String::from),
                text: a.text().collect::<String>().trim().to_string(),
                rel: attr(&a, "rel").unwrap_or_default().to_lowercase(),
                href,
            }
        })
        .collect();
    page.images = dom
        .select(&sel.images)
        .map(|i| {
            let src = attr(&i, "src").unwrap_or_default();
            Image {
                url: (!src.is_empty()).then(|| abs(&src)).flatten().map(String::from),
                src,
                alt: attr(&i, "alt"),
                width: attr(&i, "width"),
                height: attr(&i, "height"),
                loading: attr(&i, "loading"),
                srcset: attr(&i, "srcset"),
            }
        })
        .collect();
    page.scripts = dom
        .select(&sel.scripts)
        .map(|s| Script {
            src: attr(&s, "src").and_then(|src| abs(&src)).map(String::from),
            inline_bytes: s.text().map(str::len).sum::<usize>() as u64,
            is_async: s.value().attr("async").is_some(),
            defer: s.value().attr("defer").is_some(),
        })
        .collect();
    page.stylesheets =
        dom.select(&sel.stylesheets).filter_map(|l| attr(&l, "href").and_then(|h| abs(&h)).map(String::from)).collect();
    page.text = dom.select(&sel.body).next().map_or_else(|| visible_text(root), visible_text);
}

/// Element text with whitespace collapsed, the way cheerio's `.text().trim()` reads it.
fn text_of(e: ElementRef) -> String {
    collapse(&e.text().collect::<String>())
}

/// Text outside script, style, noscript and template elements.
fn visible_text(e: ElementRef) -> String {
    let mut out = String::new();
    for node in e.descendants() {
        let Some(text) = node.value().as_text() else { continue };
        let hidden = node
            .parent()
            .and_then(|p| {
                p.value().as_element().map(|el| matches!(el.name(), "script" | "style" | "noscript" | "template"))
            })
            .unwrap_or(false);
        if !hidden {
            out.push_str(text);
        }
    }
    collapse(&out)
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_basics() {
        let html = r#"<!doctype html><html lang="en"><head><title> Hi  there </title>
            <meta name="Description" content="d"><meta property="og:title" content="o">
            <link rel="canonical" href="/c"><link rel="stylesheet" href="/s.css">
            <script src="/a.js" async></script><script>var x = 1;</script></head>
            <body><h1>Top</h1><h2>Sub <b>part</b></h2>
            <a href="/in#x">In</a><a href="https://other.test/">Out</a><a href="mailto:a@b.c">Mail</a>
            <img src="i.png" alt="" width="3"><style>.x{}</style><p>Body   text</p></body></html>"#;
        let mut page = PageData { final_url: "https://site.test/dir/page".into(), ..Default::default() };
        let origin = Url::parse("https://site.test/").unwrap().origin();
        extract(&Html::parse_document(html), &mut page, &origin);
        assert_eq!(page.title.as_deref(), Some("Hi there"));
        assert_eq!(page.lang.as_deref(), Some("en"));
        assert_eq!(page.meta[0].name, "description");
        assert_eq!(page.canonical.as_deref(), Some("/c"));
        assert_eq!(page.headings.len(), 2);
        assert_eq!(page.headings[1].text, "Sub part");
        assert_eq!(page.links[0].url.as_deref(), Some("https://site.test/in#x"));
        assert!(page.links[0].internal && !page.links[1].internal);
        assert_eq!(page.links[2].url.as_deref(), Some("mailto:a@b.c"));
        assert!(!page.links[2].internal);
        assert_eq!(page.images[0].url.as_deref(), Some("https://site.test/dir/i.png"));
        assert_eq!(page.images[0].alt.as_deref(), Some(""));
        assert_eq!(page.scripts.len(), 2);
        assert!(page.scripts[0].is_async && page.scripts[1].src.is_none());
        assert_eq!(page.stylesheets, ["https://site.test/s.css"]);
        assert_eq!(page.text, "TopSub part InOutMail Body text");
    }
}
