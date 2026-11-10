//! Sitemap and sitemap index parsing.

use quick_xml::Reader;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

/// Sitemap URLs as fetched once per audit.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SitemapInfo {
    /// True when the first sitemap answered 2xx.
    pub exists: bool,
    pub status: u16,
    /// Page URLs from every sitemap file, index children included. At most 50,000.
    pub urls: Vec<String>,
}

/// At most this many URLs per file, and in total.
pub(crate) const URL_CAP: usize = 50_000;

pub(crate) struct Parsed {
    pub(crate) is_index: bool,
    pub(crate) locs: Vec<String>,
}

/// `<loc>` values in document order, and whether the root is `<sitemapindex>`. Stops at the
/// first XML error and keeps what it read.
pub(crate) fn parse(xml: &[u8]) -> Parsed {
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let (mut is_index, mut root_seen) = (false, false);
    // Text of the <loc> being read. Entity references arrive as separate events.
    let mut loc: Option<String> = None;
    let mut locs = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.local_name();
                if !root_seen {
                    root_seen = true;
                    is_index = name.as_ref() == "sitemapindex";
                }
                loc = (name.as_ref() == "loc").then(String::new);
            }
            Ok(Event::Text(t)) => {
                if let Some(l) = loc.as_mut() {
                    l.push_str(&t.xml10_content());
                }
            }
            Ok(Event::GeneralRef(r)) => {
                if let Some(l) = loc.as_mut() {
                    if let Ok(Some(c)) = r.resolve_char_ref() {
                        l.push(c);
                    } else {
                        l.push_str(quick_xml::escape::resolve_predefined_entity(&r.xml10_content()).unwrap_or(""));
                    }
                }
            }
            Ok(Event::End(_)) => {
                if let Some(l) = loc.take() {
                    let l = l.trim();
                    if !l.is_empty() && locs.len() < URL_CAP {
                        locs.push(l.to_string());
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Parsed { is_index, locs }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_urlsets_and_indexes() {
        let set = br#"<?xml version="1.0"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <url><loc> https://s.test/a?x=1&amp;y=2 </loc></url><url><loc>https://s.test/b</loc></url></urlset>"#;
        let p = super::parse(set);
        assert!(!p.is_index);
        assert_eq!(p.locs, ["https://s.test/a?x=1&y=2", "https://s.test/b"]);
        let index = br#"<sitemapindex><sitemap><loc>https://s.test/s1.xml</loc></sitemap></sitemapindex>"#;
        let p = super::parse(index);
        assert!(p.is_index);
        assert_eq!(p.locs, ["https://s.test/s1.xml"]);
    }
}
