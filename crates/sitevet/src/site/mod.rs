//! Site-wide analysis (FR-SITE-01 to FR-SITE-06): checks that compare pages, run once after the
//! crawl over a small summary of each page. Each finding lists the pages it affects, so the
//! overall score can scale its penalty by the share of the site it touches.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use url::Url;

use crate::report::{Finding, Metric, MetricCell, Severity, SiteFinding, SiteResult, Status, ValueStatus};
use crate::{BlockedUrl, MetricInput, PageData, SiteData};

/// Pairs at most this many of 256 SimHash bits apart are near-duplicates: 80% similar or more.
/// One changed paragraph in twelve moves it about 42 bits; unrelated pages sit near 128.
const NEAR_DUP_BITS: u32 = 51;
/// Site tables keep more rows than page tables: they are the whole site.
const SITE_ROWS: usize = 100;

/// What the site checks need from one crawled page.
pub(crate) struct SitePage {
    url: String,
    final_url: String,
    status: u16,
    /// 2xx without a fetch error.
    ok: bool,
    title: Option<String>,
    description: Option<String>,
    h1: Option<String>,
    /// Internal http(s) links, fragment dropped.
    links: Vec<String>,
    simhash: Option<[u64; 4]>,
    /// No noindex, and no canonical pointing at another URL.
    indexable: bool,
}

fn norm(u: &str) -> String {
    Url::parse(u).map_or_else(|_| u.to_string(), |u| crate::crawl::normalize(&u).into())
}

fn clean(s: Option<&str>) -> Option<String> {
    s.map(|t| t.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|t| !t.is_empty())
}

impl SitePage {
    pub(crate) fn new(p: &PageData) -> Self {
        let meta = |name: &str| p.meta.iter().find(|m| m.name == name).map(|m| m.content.as_str());
        let x_robots = p.headers.iter().filter(|(k, _)| k == "x-robots-tag").map(|(_, v)| v.as_str());
        let noindex = meta("robots").into_iter().chain(x_robots).any(|v| v.to_lowercase().contains("noindex"));
        let canonical_elsewhere = p.canonical.as_deref().is_some_and(|c| {
            Url::parse(&p.final_url)
                .and_then(|b| b.join(c.trim()))
                .is_ok_and(|c| norm(c.as_str()) != norm(&p.final_url))
        });
        Self {
            url: norm(&p.url),
            final_url: norm(&p.final_url),
            status: p.status,
            ok: p.error.is_none() && (200..300).contains(&p.status),
            title: clean(p.title.as_deref()),
            description: clean(meta("description")),
            h1: clean(p.headings.iter().find(|h| h.level == 1).map(|h| h.text.as_str())),
            links: p
                .links
                .iter()
                .filter(|l| l.internal)
                .filter_map(|l| l.url.as_deref())
                .filter(|u| u.starts_with("http"))
                .map(norm)
                .collect(),
            simhash: p.simhash,
            indexable: !noindex && !canonical_elsewhere,
        }
    }
}

pub(crate) struct Input<'a> {
    pub(crate) pages: &'a [SitePage],
    pub(crate) redirects: &'a [(String, String)],
    pub(crate) blocked: &'a [BlockedUrl],
    pub(crate) site: &'a SiteData,
    pub(crate) start: &'a str,
    pub(crate) click_depth_warn: u32,
    /// Categories to keep, from `only` and `skip`.
    pub(crate) wanted: &'a dyn Fn(&str) -> bool,
}

struct Check {
    id: &'static str,
    title: &'static str,
    severity: Severity,
    fix: &'static str,
}

impl Check {
    fn category(&self) -> &'static str {
        self.id.split('/').next().unwrap_or(self.id)
    }

    fn finding(&self, status: Status, message: String, evidence: Option<String>, urls: Vec<String>) -> SiteFinding {
        let key_part = evidence.clone().or_else(|| urls.first().cloned());
        SiteFinding {
            finding: Finding {
                rule_id: self.id.to_string(),
                category: self.category().to_string(),
                title: self.title.to_string(),
                status,
                severity: self.severity,
                message,
                details: None,
                evidence,
                fix: Some(self.fix.to_string()),
                key: crate::engine::pipeline::finding_key(self.id, "site", key_part.as_deref()),
            },
            urls,
        }
    }

    fn pass(&self, message: impl Into<String>) -> SiteFinding {
        self.finding(Status::Pass, message.into(), None, Vec::new())
    }
}

const DUP_TITLE: Check = Check {
    id: "core-seo/duplicate-title",
    title: "Duplicate titles",
    severity: Severity::Warning,
    fix: "Give every page its own title that says what that page is about.",
};
const DUP_DESCRIPTION: Check = Check {
    id: "core-seo/duplicate-description",
    title: "Duplicate meta descriptions",
    severity: Severity::Warning,
    fix: "Write a meta description for each page, or leave it out and let search engines pick a snippet.",
};
const DUP_H1: Check = Check {
    id: "content/duplicate-h1",
    title: "Duplicate H1 headings",
    severity: Severity::Info,
    fix: "Give each page an H1 that names its own topic.",
};
const NEAR_DUP: Check = Check {
    id: "content/near-duplicate",
    title: "Near-duplicate content",
    severity: Severity::Warning,
    fix: "Merge near-identical pages, point one at the other with a canonical link, or rewrite one of them.",
};
const ORPHANS: Check = Check {
    id: "crawlability/orphan-pages",
    title: "Orphan pages",
    severity: Severity::Warning,
    fix: "Link to each orphan page from at least one related page, or drop it from the sitemap.",
};
const DEPTH: Check = Check {
    id: "crawlability/click-depth",
    title: "Click depth",
    severity: Severity::Warning,
    fix: "Link deep pages from higher up: the home page, a hub page or the navigation.",
};
const REDIRECTS: Check = Check {
    id: "links/internal-redirects",
    title: "Internal links to redirects",
    severity: Severity::Info,
    fix: "Point internal links straight at the final URL, so visitors and crawlers skip the hop.",
};
const SITEMAP_BAD: Check = Check {
    id: "crawlability/sitemap-not-crawlable",
    title: "Sitemap URLs that can't be indexed",
    severity: Severity::Warning,
    fix: "List only live, indexable, final URLs in the sitemap: no redirects, errors, noindex or blocked paths.",
};
const SITEMAP_MISSING: Check = Check {
    id: "crawlability/missing-from-sitemap",
    title: "Pages missing from the sitemap",
    severity: Severity::Info,
    fix: "Add every indexable page to the sitemap.",
};

fn metric(m: MetricInput) -> Metric {
    Metric { id: m.id, category: "site".into(), label: m.label, status: m.status, note: m.note, body: m.body }
}

fn table(id: &str, label: &str, columns: &[&str], rows: Vec<Vec<MetricCell>>) -> MetricInput {
    MetricInput::table_capped(id, label, columns, rows, SITE_ROWS)
}

fn cell(s: impl Into<String>) -> MetricCell {
    MetricCell::Text(s.into())
}

fn num(n: impl Into<f64>) -> MetricCell {
    MetricCell::Number(n.into())
}

/// The link graph: which pages link to which, with redirects followed.
struct Graph {
    /// Distinct pages linking to each page.
    inbound: Vec<usize>,
    outbound: Vec<usize>,
    /// Clicks from the start page. None when no chain of links reaches the page.
    depth: Vec<Option<u32>>,
    /// Crawled pages linking to each URL as written, before redirects.
    linked_from: HashMap<String, usize>,
}

fn graph(input: &Input) -> Graph {
    let pages = input.pages;
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (i, p) in pages.iter().enumerate() {
        index.entry(&p.final_url).or_insert(i);
        index.entry(&p.url).or_insert(i);
    }
    let hops: HashMap<&str, &str> = input.redirects.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let resolve = |u: &str| index.get(u).or_else(|| hops.get(u).and_then(|t| index.get(t))).copied();

    let mut edges: Vec<HashSet<usize>> = vec![HashSet::new(); pages.len()];
    let mut linked_from: HashMap<String, usize> = HashMap::new();
    for (i, p) in pages.iter().enumerate() {
        let targets: HashSet<&String> = p.links.iter().collect();
        for t in targets {
            *linked_from.entry(t.clone()).or_default() += 1;
            if let Some(j) = resolve(t).filter(|j| *j != i) {
                edges[i].insert(j);
            }
        }
    }
    let mut inbound = vec![0; pages.len()];
    for e in &edges {
        for j in e {
            inbound[*j] += 1;
        }
    }
    let mut depth = vec![None; pages.len()];
    if let Some(s) = resolve(&norm(input.start)) {
        depth[s] = Some(0);
        let mut queue = VecDeque::from([s]);
        while let Some(i) = queue.pop_front() {
            for &j in &edges[i] {
                if depth[j].is_none() {
                    depth[j] = depth[i].map(|d| d + 1);
                    queue.push_back(j);
                }
            }
        }
    }
    Graph { inbound, outbound: edges.iter().map(HashSet::len).collect(), depth, linked_from }
}

/// Groups of two or more ok pages sharing a value, largest first.
fn duplicates(pages: &[SitePage], field: impl Fn(&SitePage) -> Option<&String>) -> Vec<(String, Vec<String>)> {
    let mut groups: BTreeMap<&String, Vec<String>> = BTreeMap::new();
    for p in pages.iter().filter(|p| p.ok) {
        if let Some(v) = field(p) {
            groups.entry(v).or_default().push(p.final_url.clone());
        }
    }
    let mut out: Vec<(String, Vec<String>)> =
        groups.into_iter().filter(|(_, u)| u.len() > 1).map(|(v, u)| (v.clone(), u)).collect();
    out.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
    out
}

/// `article` is "a" or "an", for the pass message.
fn duplicate_findings(check: &Check, article: &str, what: &str, groups: &[(String, Vec<String>)]) -> Vec<SiteFinding> {
    if groups.is_empty() {
        return vec![check.pass(format!("No two pages share {article} {what}"))];
    }
    groups
        .iter()
        .map(|(value, urls)| {
            check.finding(
                Status::Warn,
                format!("{} pages share the {what} \"{}\"", urls.len(), crate::rules::kit::clip(value, 80)),
                Some(value.clone()),
                urls.clone(),
            )
        })
        .collect()
}

/// "1 page is" or "3 pages are".
fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn similarity(bits: u32) -> u32 {
    ((256 - bits) * 100 + 128) / 256
}

/// Near-duplicate pairs (i, j, similarity percent), most similar first.
// ponytail: every pair is compared, 50 million XOR-and-popcounts at 10,000 pages (about 0.2 s).
// Band the hashes into buckets first if max_pages ever grows past that.
fn near_duplicates(pages: &[SitePage]) -> Vec<(usize, usize, u32)> {
    let hashed: Vec<(usize, [u64; 4])> =
        pages.iter().enumerate().filter(|(_, p)| p.ok).filter_map(|(i, p)| p.simhash.map(|h| (i, h))).collect();
    let mut pairs = Vec::new();
    for (k, (a, ha)) in hashed.iter().enumerate() {
        for (b, hb) in &hashed[k + 1..] {
            let bits: u32 = ha.iter().zip(hb).map(|(x, y)| (x ^ y).count_ones()).sum();
            if bits <= NEAR_DUP_BITS {
                pairs.push((*a, *b, similarity(bits)));
            }
        }
    }
    pairs.sort_by(|a, b| b.2.cmp(&a.2).then((a.0, a.1).cmp(&(b.0, b.1))));
    pairs
}

pub(crate) fn analyze(input: &Input) -> SiteResult {
    let pages = input.pages;
    let g = graph(input);
    let mut findings: Vec<SiteFinding> = Vec::new();
    let mut metrics: Vec<MetricInput> = Vec::new();

    // FR-SITE-01: duplicate titles, descriptions and H1s.
    let titles = duplicates(pages, |p| p.title.as_ref());
    let descriptions = duplicates(pages, |p| p.description.as_ref());
    let h1s = duplicates(pages, |p| p.h1.as_ref());
    findings.extend(duplicate_findings(&DUP_TITLE, "a", "title", &titles));
    findings.extend(duplicate_findings(&DUP_DESCRIPTION, "a", "meta description", &descriptions));
    findings.extend(duplicate_findings(&DUP_H1, "an", "H1", &h1s));
    let rows: Vec<Vec<MetricCell>> = [("Title", &titles), ("Description", &descriptions), ("H1", &h1s)]
        .into_iter()
        .flat_map(|(field, groups)| {
            groups.iter().map(move |(v, urls)| {
                vec![cell(field), cell(crate::rules::kit::clip(v, 70)), num(urls.len() as f64), cell(urls.join(" "))]
            })
        })
        .collect();
    metrics.push(table("site/duplicate-groups", "Duplicate groups", &["Field", "Value", "Pages", "URLs"], rows));

    // FR-SITE-02: near-duplicate content.
    let pairs = near_duplicates(pages);
    if pairs.is_empty() {
        findings.push(NEAR_DUP.pass("No near-duplicate pages"));
    }
    for (a, b, pct) in &pairs {
        let urls = vec![pages[*a].final_url.clone(), pages[*b].final_url.clone()];
        findings.push(NEAR_DUP.finding(Status::Warn, format!("2 pages are {pct}% similar"), None, urls));
    }
    metrics.push(
        table(
            "site/near-duplicates",
            "Near-duplicate pages",
            &["Page", "Similar page", "Similarity %"],
            pairs
                .iter()
                .map(|(a, b, pct)| vec![cell(&pages[*a].final_url), cell(&pages[*b].final_url), num(*pct)])
                .collect(),
        )
        .with_note(format!(
            "Main content compared with a 256-bit SimHash over 5-word shingles. Pairs {}% similar or more are listed",
            similarity(NEAR_DUP_BITS)
        )),
    );

    // FR-SITE-04: the link graph and click depth.
    let mut order: Vec<usize> = (0..pages.len()).collect();
    order.sort_by(|a, b| g.inbound[*b].cmp(&g.inbound[*a]).then(pages[*a].final_url.cmp(&pages[*b].final_url)));
    metrics.push(
        table(
            "site/link-graph",
            "Internal link graph",
            &["URL", "Linked from", "Links to", "Click depth"],
            order
                .iter()
                .map(|&i| {
                    vec![
                        cell(&pages[i].final_url),
                        num(g.inbound[i] as f64),
                        num(g.outbound[i] as f64),
                        g.depth[i].map_or_else(|| cell("not linked"), num),
                    ]
                })
                .collect(),
        )
        .with_note("Linked from counts distinct crawled pages. Click depth counts clicks from the start page"),
    );
    let mut by_depth: BTreeMap<Option<u32>, usize> = BTreeMap::new();
    for d in &g.depth {
        *by_depth.entry(*d).or_default() += 1;
    }
    metrics.push(
        table(
            "site/click-depth",
            "Pages by click depth",
            &["Click depth", "Pages"],
            by_depth
                .iter()
                .filter(|(d, _)| d.is_some())
                .chain(by_depth.iter().filter(|(d, _)| d.is_none()))
                .map(|(d, n)| vec![d.map_or_else(|| cell("not linked"), num), num(*n as f64)])
                .collect(),
        )
        .with_note(format!("Deeper than {} warns", input.click_depth_warn)),
    );
    let deep: Vec<String> = pages
        .iter()
        .zip(&g.depth)
        .filter(|(_, d)| d.is_some_and(|d| d > input.click_depth_warn))
        .map(|(p, _)| p.final_url.clone())
        .collect();
    findings.push(if deep.is_empty() {
        DEPTH.pass(format!("Every linked page is within {} clicks of the start page", input.click_depth_warn))
    } else {
        DEPTH.finding(
            Status::Warn,
            format!(
                "{} more than {} clicks from the start page",
                count(deep.len(), "page is", "pages are"),
                input.click_depth_warn
            ),
            None,
            deep,
        )
    });

    // FR-SITE-05: internal links that land on a redirect.
    let mut hops: Vec<(&String, &String, usize)> =
        input.redirects.iter().filter_map(|(from, to)| g.linked_from.get(from).map(|n| (from, to, *n))).collect();
    hops.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(b.0)));
    hops.dedup_by(|a, b| a.0 == b.0);
    findings.push(if hops.is_empty() {
        REDIRECTS.pass("No internal links point at a redirect")
    } else {
        REDIRECTS.finding(
            Status::Warn,
            count(hops.len(), "internal link target redirects", "internal link targets redirect"),
            None,
            hops.iter().map(|(from, _, _)| (*from).clone()).collect(),
        )
    });
    metrics.push(table(
        "site/redirect-map",
        "Redirect map",
        &["Link", "Ends at", "Linked from"],
        hops.iter().map(|(from, to, n)| vec![cell(*from), cell(*to), num(*n as f64)]).collect(),
    ));

    // FR-SITE-03 and FR-SITE-06 need a sitemap.
    let sitemap = &input.site.sitemap;
    if !sitemap.exists {
        for check in [&ORPHANS, &SITEMAP_BAD, &SITEMAP_MISSING] {
            findings.push(check.finding(Status::Info, "No sitemap to compare with".into(), None, Vec::new()));
        }
    } else {
        let listed: Vec<String> = sitemap.urls.iter().map(|u| norm(u)).collect();
        let in_sitemap: HashSet<&str> = listed.iter().map(String::as_str).collect();
        let start = norm(input.start);

        let orphans: Vec<String> = pages
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                p.ok && g.inbound[*i] == 0
                    && p.url != start
                    && p.final_url != start
                    && (in_sitemap.contains(p.url.as_str()) || in_sitemap.contains(p.final_url.as_str()))
            })
            .map(|(_, p)| p.final_url.clone())
            .collect();
        findings.push(if orphans.is_empty() {
            ORPHANS.pass("Every sitemap page is linked from another page")
        } else {
            ORPHANS.finding(
                Status::Warn,
                format!(
                    "{} no internal links pointing at {}",
                    count(orphans.len(), "sitemap page has", "sitemap pages have"),
                    if orphans.len() == 1 { "it" } else { "them" }
                ),
                None,
                orphans.clone(),
            )
        });
        metrics.push(table("site/orphans", "Orphan pages", &["URL"], orphans.iter().map(|u| vec![cell(u)]).collect()));

        let blocked: HashSet<String> = input.blocked.iter().map(|b| norm(&b.url)).collect();
        let by_url: HashMap<&str, &SitePage> = pages.iter().map(|p| (p.url.as_str(), p)).collect();
        let hop: HashMap<&str, &str> = input.redirects.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let mut bad: Vec<(String, String)> = Vec::new();
        for u in &listed {
            let reason = if blocked.contains(u) {
                Some("blocked by robots.txt".to_string())
            } else if let Some(to) = hop.get(u.as_str()) {
                Some(format!("redirects to {to}"))
            } else if let Some(p) = by_url.get(u.as_str()) {
                if !p.ok {
                    Some(if p.status == 0 { "no response".to_string() } else { format!("status {}", p.status) })
                } else if !p.indexable {
                    Some("noindex, or a canonical pointing elsewhere".to_string())
                } else {
                    None
                }
            } else {
                None // not crawled: past max_pages or max_depth, so unknown
            };
            if let Some(r) = reason {
                bad.push((u.clone(), r));
            }
        }
        findings.push(if bad.is_empty() {
            SITEMAP_BAD.pass("Every crawled sitemap URL is live and indexable")
        } else {
            SITEMAP_BAD.finding(
                Status::Warn,
                format!("{} be indexed as listed", count(bad.len(), "sitemap URL can't", "sitemap URLs can't")),
                None,
                bad.iter().map(|(u, _)| u.clone()).collect(),
            )
        });
        metrics.push(table(
            "site/sitemap-not-crawlable",
            "In the sitemap, not indexable",
            &["URL", "Why"],
            bad.iter().map(|(u, r)| vec![cell(u), cell(r)]).collect(),
        ));

        let missing: Vec<String> = pages
            .iter()
            .filter(|p| {
                p.ok && p.indexable
                    && !in_sitemap.contains(p.url.as_str())
                    && !in_sitemap.contains(p.final_url.as_str())
            })
            .map(|p| p.final_url.clone())
            .collect();
        findings.push(if missing.is_empty() {
            SITEMAP_MISSING.pass("Every indexable crawled page is in the sitemap")
        } else {
            SITEMAP_MISSING.finding(
                Status::Warn,
                format!("{} not in the sitemap", count(missing.len(), "indexable page is", "indexable pages are")),
                None,
                missing.clone(),
            )
        });
        metrics.push(table(
            "site/missing-from-sitemap",
            "Crawlable, not in the sitemap",
            &["URL"],
            missing.iter().map(|u| vec![cell(u)]).collect(),
        ));
    }

    findings.retain(|f| (input.wanted)(&f.finding.category));
    let metrics = if (input.wanted)("site") {
        metrics
            .into_iter()
            .map(|m| {
                let empty = matches!(&m.body, crate::MetricBody::Table { total: 0, .. });
                metric(if empty { m.with_status(ValueStatus::Ok) } else { m })
            })
            .collect()
    } else {
        Vec::new()
    };
    SiteResult { findings, metrics }
}

#[cfg(test)]
mod tests;
