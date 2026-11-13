//! FR-SITE-01 to FR-SITE-06 on one fixture site with every site-wide issue built in once.

#![allow(clippy::unwrap_used)]

mod support;

use sitevet::{AuditReport, MetricBody, MetricCell, Options, SiteFinding, Status, audit};
use support::{Fixture, Route, Site};

const BREAD: [&str; 12] = [
    "Good bread starts with flour, water, salt and a little patience on a quiet morning.",
    "Mix the dough until it holds together, then leave it covered for twenty minutes.",
    "Folding the dough every half hour builds strength without hard kneading on the counter.",
    "A warm kitchen speeds the first rise, while a cool one gives a deeper flavour overnight.",
    "Shape the loaf gently so the bubbles inside survive the move into the basket.",
    "Dust the basket with rice flour, because wheat flour tends to stick to damp dough.",
    "Heat a heavy pot in the oven for forty minutes before the loaf goes in.",
    "Score the top with a sharp blade so steam escapes along a line you chose.",
    "Bake with the lid on first, then take it off to let the crust darken.",
    "Tap the bottom of the loaf: a hollow sound means the inside has finished baking.",
    "Let the bread cool on a rack for an hour, or the crumb turns gummy.",
    "Store the loaf cut side down on a board and it keeps for two days.",
];

const GARDEN: &str = "Tomatoes need full sun, deep watering twice a week and a stake before the stems get heavy. \
    Pinch the side shoots that grow between the main stem and the branches. Mulch keeps the soil damp and the roots \
    cool through summer. Feed the plants with compost when the first fruit sets, then every two weeks after that. \
    Pick the fruit when it turns fully red and gives slightly under a gentle squeeze.";

fn page(title: &str, body: &str) -> String {
    format!(
        "<html lang=\"en\"><head><title>{title}</title></head><body><main><h1>{title}</h1>{body}</main></body></html>"
    )
}

fn links(paths: &[&str]) -> String {
    paths.iter().map(|p| format!("<a href=\"{p}\">{p}</a> ")).collect()
}

fn bread(changed: Option<&str>) -> String {
    BREAD.iter().enumerate().map(|(i, s)| format!("<p>{}</p>", if i == 5 { changed.unwrap_or(s) } else { s })).collect()
}

/// `/c` is crawlable but missing from the sitemap. `/old` is in the sitemap but redirects.
/// `/orphan` is only in the sitemap. `/deep3` is 4 clicks from the start page.
fn fixture_site() -> Site {
    let sitemap: String = ["/", "/a", "/b", "/new", "/old", "/dup1", "/dup2", "/deep1", "/deep2", "/deep3", "/orphan"]
        .iter()
        .map(|p| format!("<url><loc>{{{{origin}}}}{p}</loc></url>"))
        .collect();
    Site::new()
        .page("/", page("Home", &links(&["/a", "/b", "/c", "/old", "/dup1"])))
        .page("/a", page("Shared title", &format!("<p>{GARDEN}</p>{}", links(&["/b", "/deep1"]))))
        .page("/b", page("Shared title", &links(&["/"])))
        .page("/c", page("Shared title", &links(&["/a"])))
        .route("/old", Route::redirect("/new"))
        .page("/new", page("New page", ""))
        .page("/dup1", page("Bread at home", &format!("{}{}", bread(None), links(&["/dup2"]))))
        .page(
            "/dup2",
            page(
                "Home bread",
                &bread(Some("Semolina in the basket works too, and it gives the crust a golden colour and a crunch.")),
            ),
        )
        .page("/deep1", page("Deep one", &links(&["/deep2"])))
        .page("/deep2", page("Deep two", &links(&["/deep3"])))
        .page("/deep3", page("Deep three", ""))
        .page("/orphan", page("Orphan", ""))
        .route("/sitemap.xml", Route::html(format!("<urlset>{sitemap}</urlset>")).content_type("application/xml"))
}

fn findings<'a>(r: &'a AuditReport, id: &str) -> Vec<&'a SiteFinding> {
    r.site.findings.iter().filter(|f| f.finding.rule_id == id).collect()
}

fn paths(f: &SiteFinding) -> Vec<String> {
    let mut p: Vec<String> = f.urls.iter().map(|u| url::Url::parse(u).unwrap().path().to_string()).collect();
    p.sort();
    p
}

fn table(r: &AuditReport, id: &str) -> Vec<Vec<MetricCell>> {
    match &r.site.metrics.iter().find(|m| m.id == id).unwrap().body {
        MetricBody::Table { rows, .. } => rows.clone(),
        _ => panic!("{id} is not a table"),
    }
}

fn text(c: &MetricCell) -> String {
    match c {
        MetricCell::Text(t) => url::Url::parse(t).map_or_else(|_| t.clone(), |u| u.path().to_string()),
        MetricCell::Number(n) => n.to_string(),
    }
}

async fn run() -> AuditReport {
    let fx = Fixture::start(fixture_site()).await;
    audit(&fx.url("/"), Options::default()).await.unwrap()
}

#[tokio::test]
async fn site_wide_findings_on_the_fixture_site() {
    let r = run().await;

    // FR-SITE-01: one group of 3 pages sharing a title.
    let titles = findings(&r, "core-seo/duplicate-title");
    assert_eq!(titles.len(), 1);
    assert_eq!(titles[0].finding.status, Status::Warn);
    assert_eq!(paths(titles[0]), ["/a", "/b", "/c"]);
    assert_eq!(titles[0].finding.message, "3 pages share the title \"Shared title\"");

    // FR-SITE-02: the two bread pages differ by one paragraph; the garden page is unrelated.
    let near = findings(&r, "content/near-duplicate");
    assert_eq!(near.len(), 1, "{near:?}");
    assert_eq!(paths(near[0]), ["/dup1", "/dup2"]);

    // FR-SITE-03: the sitemap-only page.
    let orphans = findings(&r, "crawlability/orphan-pages");
    assert_eq!(paths(orphans[0]), ["/orphan"]);

    // FR-SITE-04: click depth past 3.
    assert_eq!(paths(findings(&r, "crawlability/click-depth")[0]), ["/deep3"]);

    // FR-SITE-05: /old and where it ends.
    assert_eq!(paths(findings(&r, "links/internal-redirects")[0]), ["/old"]);
    assert_eq!(
        table(&r, "site/redirect-map").iter().map(|row| row.iter().map(text).collect()).collect::<Vec<Vec<_>>>(),
        [["/old", "/new", "1"]]
    );

    // FR-SITE-06: one entry each way.
    assert_eq!(paths(findings(&r, "crawlability/sitemap-not-crawlable")[0]), ["/old"]);
    assert_eq!(paths(findings(&r, "crawlability/missing-from-sitemap")[0]), ["/c"]);

    // Every site finding sits in a category score.
    assert!(r.categories.iter().any(|c| c.category == "crawlability" && c.warn >= 3));
}

/// FR-SITE-04: inbound counts and click depth match the known graph.
#[tokio::test]
async fn link_graph_matches_the_known_graph() {
    let r = run().await;
    let graph: Vec<(String, String, String)> =
        table(&r, "site/link-graph").iter().map(|row| (text(&row[0]), text(&row[1]), text(&row[3]))).collect();
    let row = |p: &str| graph.iter().find(|(u, _, _)| u == p).unwrap().clone();
    // / is linked from /b; /a from / and /c; /b from / and /a; /new from / (through /old).
    assert_eq!(row("/"), ("/".into(), "1".into(), "0".into()));
    assert_eq!(row("/a"), ("/a".into(), "2".into(), "1".into()));
    assert_eq!(row("/b"), ("/b".into(), "2".into(), "1".into()));
    assert_eq!(row("/new"), ("/new".into(), "1".into(), "1".into()));
    assert_eq!(row("/deep3"), ("/deep3".into(), "1".into(), "4".into()));
    assert_eq!(row("/orphan"), ("/orphan".into(), "0".into(), "not linked".into()));
}

/// `only` keeps site findings and metrics to the chosen categories.
#[tokio::test]
async fn only_filters_site_checks() {
    let mut only = Options::default();
    only.only = vec!["security".into()];
    let fx = Fixture::start(fixture_site()).await;
    let r = audit(&fx.url("/"), only).await.unwrap();
    assert!(r.site.findings.is_empty() && r.site.metrics.is_empty());
    let mut crawl = Options::default();
    crawl.only = vec!["crawlability".into()];
    let r = audit(&fx.url("/"), crawl).await.unwrap();
    assert!(r.site.findings.iter().all(|f| f.finding.category == "crawlability"));
    assert!(!r.site.findings.is_empty());
}
