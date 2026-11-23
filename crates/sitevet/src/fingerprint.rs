//! Tech stack fingerprinting (FR-TECH-01 to FR-TECH-03): vendor signatures from
//! `data/fingerprints.toml`, matched against script URLs, the raw HTML, response headers,
//! cookies and meta tags. Adding a vendor is one entry in that file.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::{Regex, RegexSet};
use serde::{Deserialize, Serialize};

use crate::PageData;

/// One vendor found on a page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TechHit {
    pub name: String,
    /// analytics, ads, cms, framework or cdn.
    pub category: String,
    /// What matched, for example "script https://..." or "header cf-ray".
    pub evidence: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    vendor: Vec<Signature>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Signature {
    name: String,
    category: String,
    #[serde(default)]
    script: Vec<String>,
    #[serde(default)]
    html: Vec<String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    cookies: Vec<String>,
    #[serde(default)]
    meta: BTreeMap<String, String>,
}

struct Vendor {
    name: String,
    category: String,
    script: Option<RegexSet>,
    html: Option<RegexSet>,
    headers: Vec<(String, Regex)>,
    cookies: Option<RegexSet>,
    meta: Vec<(String, Regex)>,
}

pub(crate) struct Matcher {
    vendors: Vec<Vendor>,
}

const CATEGORIES: [&str; 5] = ["analytics", "ads", "cms", "framework", "cdn"];

fn set(patterns: &[String], vendor: &str) -> Result<Option<RegexSet>, String> {
    if patterns.is_empty() {
        return Ok(None);
    }
    RegexSet::new(patterns).map(Some).map_err(|e| format!("{vendor}: {e}"))
}

fn pairs(map: &BTreeMap<String, String>, vendor: &str) -> Result<Vec<(String, Regex)>, String> {
    map.iter().map(|(k, v)| Ok((k.to_lowercase(), Regex::new(v).map_err(|e| format!("{vendor}: {e}"))?))).collect()
}

impl Matcher {
    /// Reads a signature file. Bad regexes and unknown keys or categories are errors.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let file: File = toml::from_str(text).map_err(|e| e.to_string())?;
        let vendors = file
            .vendor
            .into_iter()
            .map(|s| {
                if !CATEGORIES.contains(&s.category.as_str()) {
                    return Err(format!("{}: unknown category `{}`", s.name, s.category));
                }
                Ok(Vendor {
                    script: set(&s.script, &s.name)?,
                    html: set(&s.html, &s.name)?,
                    headers: pairs(&s.headers, &s.name)?,
                    cookies: set(&s.cookies, &s.name)?,
                    meta: pairs(&s.meta, &s.name)?,
                    name: s.name,
                    category: s.category,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self { vendors })
    }

    /// Every vendor whose signature matches the page, in file order, with the first match as
    /// evidence.
    pub(crate) fn detect(&self, page: &PageData, html: &str) -> Vec<TechHit> {
        let scripts: Vec<&str> = page.scripts.iter().filter_map(|s| s.src.as_deref()).collect();
        let cookies: Vec<&str> = page
            .headers
            .iter()
            .filter(|(k, _)| k == "set-cookie")
            .filter_map(|(_, v)| v.split('=').next())
            .map(str::trim)
            .collect();
        self.vendors
            .iter()
            .filter_map(|v| {
                let evidence =
                    v.script
                        .as_ref()
                        .and_then(|s| scripts.iter().find(|u| s.is_match(u)).map(|u| format!("script {u}")))
                        .or_else(|| {
                            v.headers.iter().find_map(|(name, re)| {
                                page.headers
                                    .iter()
                                    .find(|(k, val)| k == name && re.is_match(val))
                                    .map(|_| format!("header {name}"))
                            })
                        })
                        .or_else(|| {
                            v.meta.iter().find_map(|(name, re)| {
                                page.meta.iter().find(|m| m.name == *name && re.is_match(&m.content)).map(|m| {
                                    format!("meta {name} \"{}\"", m.content.chars().take(60).collect::<String>())
                                })
                            })
                        })
                        .or_else(|| {
                            v.cookies
                                .as_ref()
                                .and_then(|s| cookies.iter().find(|c| s.is_match(c)).map(|c| format!("cookie {c}")))
                        })
                        .or_else(|| v.html.as_ref().filter(|s| s.is_match(html)).map(|_| "page markup".to_string()))?;
                Some(TechHit { name: v.name.clone(), category: v.category.clone(), evidence })
            })
            .collect()
    }
}

/// The signatures shipped in the crate.
#[allow(clippy::expect_used)] // the bundled file is checked by a test
pub(crate) static BUILTIN: LazyLock<Matcher> =
    LazyLock::new(|| Matcher::parse(include_str!("../data/fingerprints.toml")).expect("valid fingerprints.toml"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_file_parses_with_every_listed_vendor() {
        let m = Matcher::parse(include_str!("../data/fingerprints.toml")).unwrap();
        assert_eq!(m.vendors.len(), 36);
        for c in CATEGORIES {
            assert!(m.vendors.iter().any(|v| v.category == c), "{c}");
        }
    }

    /// FR-TECH-03: a new vendor is one entry, with no engine change.
    #[test]
    fn a_signature_added_in_data_is_detected() {
        let m = Matcher::parse(
            r#"
            [[vendor]]
            name = "Acme Analytics"
            category = "analytics"
            script = ['cdn\.acme-stats\.test/a\.js']
            "#,
        )
        .unwrap();
        let mut page = PageData::default();
        page.scripts.push(crate::Script {
            src: Some("https://cdn.acme-stats.test/a.js".into()),
            inline_bytes: 0,
            is_async: true,
            defer: false,
        });
        let hits = m.detect(&page, "");
        assert_eq!(hits.len(), 1);
        assert_eq!(
            (hits[0].name.as_str(), hits[0].evidence.as_str()),
            ("Acme Analytics", "script https://cdn.acme-stats.test/a.js")
        );
    }

    #[test]
    fn bad_signatures_are_errors() {
        assert!(Matcher::parse("[[vendor]]\nname = \"X\"\ncategory = \"cms\"\nhtml = ['(']").is_err());
        assert!(Matcher::parse("[[vendor]]\nname = \"X\"\ncategory = \"toaster\"").is_err());
        assert!(Matcher::parse("[[vendor]]\nname = \"X\"\ncategory = \"cms\"\nscripts = []").is_err());
    }
}
