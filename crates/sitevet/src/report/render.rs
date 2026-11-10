//! Report rendering: JSON, Markdown and one self-contained HTML file.

use std::path::PathBuf;
use std::sync::LazyLock;

use askama::Template;

use super::{AuditReport, CategoryScore, Finding, Metric, MetricBody, MetricCell, PageResult, Status, ValueStatus};

/// Output format for [`render`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Format {
    Json,
    Markdown,
    Html,
}

/// Settings for [`render_with`].
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct RenderOptions {
    /// Where full-page screenshots were written, for links from the HTML report (M4).
    pub screenshot_dir: Option<PathBuf>,
}

/// Renders the report. JSON is pretty-printed and follows [`report_schema`].
pub fn render(report: &AuditReport, format: Format) -> String {
    render_with(report, format, &RenderOptions::default())
}

/// [`render`] with options.
pub fn render_with(report: &AuditReport, format: Format, _options: &RenderOptions) -> String {
    match format {
        // ponytail: serialising plain structs cannot fail; an empty string would only mean a serde bug.
        Format::Json => serde_json::to_string_pretty(report).unwrap_or_default(),
        Format::Markdown => MarkdownReport { v: View::new(report, md_escape) }.render().unwrap_or_default(),
        Format::Html => HtmlReport { v: View::new(report, str::to_string), css: CSS }.render().unwrap_or_default(),
    }
}

static SCHEMA: LazyLock<String> = LazyLock::new(|| {
    let generator = schemars::generate::SchemaSettings::draft2020_12().for_serialize().into_generator();
    let schema = generator.into_root_schema_for::<AuditReport>();
    serde_json::to_string_pretty(&schema).unwrap_or_default() + "\n"
});

/// The JSON Schema of the report, as committed in `schema/report.schema.json`.
pub fn report_schema() -> &'static str {
    &SCHEMA
}

const CSS: &str = include_str!("../../assets/report.css");

/// The TS package's category names and order.
const CATEGORIES: [(&str, &str); 15] = [
    ("core-seo", "Core SEO"),
    ("content", "Content"),
    ("links", "Links"),
    ("images", "Images"),
    ("structured-data", "Structured data"),
    ("security", "Security"),
    ("crawlability", "Crawlability"),
    ("url-structure", "URL structure"),
    ("social-media", "Social media"),
    ("internationalization", "Internationalization"),
    ("legal", "Legal"),
    ("analytics", "Analytics"),
    ("eeat", "E-E-A-T"),
    ("accessibility", "Accessibility"),
    ("performance", "Performance"),
];

fn category_name(id: &str) -> String {
    CATEGORIES.iter().find(|(c, _)| *c == id).map_or_else(|| id.to_string(), |(_, n)| n.to_string())
}

/// Known categories first in their order, then the rest alphabetically.
fn category_rank(id: &str) -> (usize, String) {
    (CATEGORIES.iter().position(|(c, _)| *c == id).unwrap_or(CATEGORIES.len()), id.to_string())
}

fn status_word(s: Status) -> &'static str {
    match s {
        Status::Pass => "PASS",
        Status::Fail => "FAIL",
        Status::Warn => "WARN",
        _ => "INFO",
    }
}

fn status_class(s: Status) -> &'static str {
    match s {
        Status::Pass => "pass",
        Status::Fail => "fail",
        Status::Warn => "warn",
        _ => "info",
    }
}

fn status_rank(s: Status) -> u8 {
    match s {
        Status::Fail => 0,
        Status::Warn => 1,
        Status::Info => 2,
        _ => 3,
    }
}

/// Score band shared by the badge, the grid and the CLI: 90 and up is good, 50 and up is fair.
pub(crate) fn band(score: u8) -> (&'static str, &'static str) {
    match score {
        90..=100 => ("good", "Good"),
        50..=89 => ("fair", "Needs work"),
        _ => ("poor", "Poor"),
    }
}

fn value_status(s: Option<ValueStatus>) -> &'static str {
    match s {
        Some(ValueStatus::Ok) => "ok",
        Some(ValueStatus::Warn) => "warn",
        Some(ValueStatus::Bad) => "bad",
        Some(ValueStatus::Missing) => "missing",
        _ => "",
    }
}

fn cell_text(c: &MetricCell) -> String {
    match c {
        MetricCell::Number(n) => n.to_string(),
        MetricCell::Text(t) => t.clone(),
    }
}

/// Markdown table cell: escapes everything that could break the table or start formatting.
fn md_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' | '\r' => out.push(' '),
            '\\' | '|' | '`' | '*' | '_' | '<' | '>' | '[' | ']' | '#' | '~' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

// The view model. Text is escaped for Markdown here; askama escapes it for HTML.

struct View {
    start_url: String,
    final_url: String,
    generated_at: String,
    duration: String,
    score: u8,
    band: &'static str,
    band_label: &'static str,
    pages_count: usize,
    version: String,
    schema_version: u32,
    categories: Vec<CategoryView>,
    pages: Vec<PageView>,
    blocked: Vec<(String, String)>,
    /// (reason, rule IDs), one row per reason.
    skipped: Vec<(String, String)>,
    changes: Option<ChangesView>,
}

struct ChangesView {
    /// "run 12 (2026-11-10T09:30:00Z)" or "report.json (2026-...)".
    baseline: String,
    /// "+3", "-2" or "0".
    score_change: String,
    score_band: &'static str,
    /// (category, before, after, change), only categories whose score moved or appeared.
    categories: Vec<(String, String, String, String)>,
    /// (rule, page, message)
    new_failures: Vec<(String, String, String)>,
    fixed: Vec<(String, String, String)>,
    pages_added: usize,
    pages_removed: usize,
}

fn signed(n: i16) -> String {
    if n > 0 { format!("+{n}") } else { n.to_string() }
}

impl ChangesView {
    fn new(d: &super::ReportDiff, esc: fn(&str) -> String) -> Self {
        let b = &d.baseline;
        let name = match (b.run_id, &b.file) {
            (Some(id), _) => format!("run {id}"),
            (None, Some(f)) => f.clone(),
            (None, None) => "the baseline".to_string(),
        };
        let score = |s: Option<u8>| s.map_or_else(|| "-".to_string(), |s| s.to_string());
        let refs =
            |v: &[super::FindingRef]| v.iter().map(|f| (esc(&f.rule_id), esc(&f.url), esc(&f.message))).collect();
        Self {
            baseline: esc(&format!("{name} ({})", b.generated_at)),
            score_change: signed(d.score_change),
            score_band: match d.score_change {
                1.. => "good",
                0 => "fair",
                _ => "poor",
            },
            categories: d
                .categories
                .iter()
                .filter(|c| c.before != c.after)
                .map(|c| {
                    let change = match (c.before, c.after) {
                        (Some(a), Some(b)) => signed(i16::from(b) - i16::from(a)),
                        (None, _) => "new".into(),
                        (_, None) => "not run".into(),
                    };
                    (category_name(&c.category), score(c.before), score(c.after), change)
                })
                .collect(),
            new_failures: refs(&d.new_failures),
            fixed: refs(&d.fixed),
            pages_added: d.pages_added.len(),
            pages_removed: d.pages_removed.len(),
        }
    }
}

struct CategoryView {
    name: String,
    score: u8,
    band: &'static str,
    pass: u32,
    fail: u32,
    warn: u32,
}

struct PageView {
    anchor: String,
    url: String,
    final_url: Option<String>,
    status: u16,
    error: Option<String>,
    score: u8,
    band: &'static str,
    open_issues: usize,
    groups: Vec<FindingGroup>,
    metric_groups: Vec<MetricGroup>,
}

struct FindingGroup {
    name: String,
    issues: Vec<FindingView>,
    passed: usize,
}

struct FindingView {
    word: &'static str,
    class: &'static str,
    rule_id: String,
    title: String,
    message: String,
    evidence: Option<String>,
    fix: Option<String>,
}

struct MetricGroup {
    name: String,
    values: Vec<ValueView>,
    tables: Vec<TableView>,
}

struct ValueView {
    label: String,
    value: String,
    status: &'static str,
    note: String,
}

struct TableView {
    label: String,
    caption: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl View {
    fn new(r: &AuditReport, esc: fn(&str) -> String) -> Self {
        let (overall_band, band_label) = band(r.score);
        let mut categories: Vec<&CategoryScore> = r.categories.iter().collect();
        categories.sort_by_key(|c| category_rank(&c.category));
        Self {
            start_url: esc(&r.start_url),
            final_url: esc(&r.final_url),
            generated_at: r.generated_at.clone(),
            duration: format!("{:.1} s", r.duration_ms as f64 / 1000.0),
            score: r.score,
            band: overall_band,
            band_label,
            pages_count: r.pages.len(),
            version: r.sitevet_version.clone(),
            schema_version: r.schema_version,
            categories: categories
                .into_iter()
                .map(|c| CategoryView {
                    name: category_name(&c.category),
                    score: c.score,
                    band: band(c.score).0,
                    pass: c.pass,
                    fail: c.fail,
                    warn: c.warn,
                })
                .collect(),
            pages: r.pages.iter().enumerate().map(|(i, p)| PageView::new(i, p, esc)).collect(),
            blocked: r.blocked.iter().map(|b| (esc(&b.url), esc(b.rule.as_deref().unwrap_or("")))).collect(),
            changes: r.changes.as_ref().map(|d| ChangesView::new(d, esc)),
            skipped: {
                let mut by_reason: Vec<(String, Vec<&str>)> = Vec::new();
                for s in &r.skipped_rules {
                    match by_reason.iter_mut().find(|(reason, _)| *reason == s.reason) {
                        Some((_, ids)) => ids.push(&s.rule_id),
                        None => by_reason.push((s.reason.clone(), vec![&s.rule_id])),
                    }
                }
                by_reason.into_iter().map(|(reason, ids)| (esc(&reason), esc(&ids.join(", ")))).collect()
            },
        }
    }
}

impl PageView {
    fn new(i: usize, p: &PageResult, esc: fn(&str) -> String) -> Self {
        let mut cats: Vec<&str> = p.findings.iter().map(|f| f.category.as_str()).collect();
        cats.sort_by_key(|c| category_rank(c));
        cats.dedup();
        let groups: Vec<FindingGroup> = cats
            .into_iter()
            .map(|cat| {
                let mut issues: Vec<&Finding> =
                    p.findings.iter().filter(|f| f.category == cat && f.status != Status::Pass).collect();
                issues.sort_by_key(|f| status_rank(f.status));
                FindingGroup {
                    name: category_name(cat),
                    passed: p.findings.iter().filter(|f| f.category == cat && f.status == Status::Pass).count(),
                    issues: issues
                        .into_iter()
                        .map(|f| FindingView {
                            word: status_word(f.status),
                            class: status_class(f.status),
                            rule_id: esc(&f.rule_id),
                            title: esc(&f.title),
                            message: esc(&f.message),
                            evidence: f.evidence.as_deref().map(esc),
                            fix: f.fix.as_deref().map(esc),
                        })
                        .collect(),
                }
            })
            .collect();
        Self {
            anchor: format!("page-{}", i + 1),
            url: esc(&p.url),
            final_url: (p.final_url != p.url).then(|| esc(&p.final_url)),
            status: p.status,
            error: p.error.as_deref().map(esc),
            score: p.score,
            band: band(p.score).0,
            open_issues: groups
                .iter()
                .map(|g| g.issues.iter().filter(|f| f.word == "FAIL" || f.word == "WARN").count())
                .sum(),
            groups,
            metric_groups: metric_groups(&p.metrics, esc),
        }
    }
}

fn metric_groups(metrics: &[Metric], esc: fn(&str) -> String) -> Vec<MetricGroup> {
    let mut cats: Vec<&str> = metrics.iter().map(|m| m.category.as_str()).collect();
    cats.sort_by_key(|c| category_rank(c));
    cats.dedup();
    cats.into_iter()
        .map(|cat| {
            let mut group = MetricGroup { name: category_name(cat), values: Vec::new(), tables: Vec::new() };
            for m in metrics.iter().filter(|m| m.category == cat) {
                match &m.body {
                    MetricBody::Value { value, unit } => {
                        let value = match value {
                            None => "-".to_string(),
                            Some(MetricCell::Text(t)) if t.is_empty() => "-".to_string(),
                            Some(v) => match unit {
                                Some(u) => format!("{} {u}", cell_text(v)),
                                None => cell_text(v),
                            },
                        };
                        group.values.push(ValueView {
                            label: esc(&m.label),
                            value: esc(&value),
                            status: value_status(m.status),
                            note: esc(m.note.as_deref().unwrap_or("")),
                        });
                    }
                    MetricBody::Table { columns, rows, total } => group.tables.push(TableView {
                        label: esc(&m.label),
                        caption: if (rows.len() as u32) < *total {
                            format!("showing {} of {total}", rows.len())
                        } else {
                            String::new()
                        },
                        columns: columns.iter().map(|c| esc(c)).collect(),
                        rows: rows.iter().map(|r| r.iter().map(|c| esc(&cell_text(c))).collect()).collect(),
                    }),
                }
            }
            group
        })
        .collect()
}

#[derive(Template)]
#[template(path = "report.md", escape = "none")]
struct MarkdownReport {
    v: View,
}

#[derive(Template)]
#[template(path = "report.html")]
struct HtmlReport {
    v: View,
    css: &'static str,
}

#[cfg(test)]
mod tests {
    use super::md_escape;

    #[test]
    fn markdown_escape_keeps_tables_intact() {
        assert_eq!(md_escape("a|b\nc *x* `y` <script> [l](u)"), r"a\|b c \*x\* \`y\` \<script\> \[l\](u)");
    }
}
