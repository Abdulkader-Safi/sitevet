//! The accessibility rule, ported from the TS package's `src/rules/accessibility.ts`. It runs in
//! the rendered stage (M4) and turns every axe-core result into its own finding.

use std::sync::Arc;

use crate::{Rule, RuleResult as R, Severity, Status};

const CATEGORY: &str = "accessibility";

pub(super) fn rules() -> Vec<Arc<dyn Rule>> {
    vec![Arc::new(Axe)]
}

fn impact_severity(impact: Option<&str>) -> Severity {
    match impact.unwrap_or("moderate") {
        "critical" | "serious" => Severity::Error,
        "minor" => Severity::Info,
        _ => Severity::Warning,
    }
}

rule!(
    Axe,
    "accessibility/axe",
    "Accessibility (axe-core)",
    Warning,
    Rendered,
    "Resolve the WCAG issues flagged by axe-core. Each issue links to detailed guidance.",
    |cx| {
        let Some(data) = cx.rendered.filter(|d| d.ok) else {
            return vec![R::warn("Page could not be rendered for accessibility checks")];
        };
        let violations = data.axe.iter().map(|v| {
            let severity = impact_severity(v.impact.as_deref());
            let status = if severity == Severity::Info { Status::Warn } else { Status::Fail };
            let plural = if v.nodes == 1 { "" } else { "s" };
            let impact = v.impact.as_deref().unwrap_or("unknown");
            let mut r = R::new(status, format!("{} ({} element{plural}, impact: {impact})", v.description, v.nodes));
            r.rule_id = Some(format!("accessibility/{}", v.id));
            r.title = Some(v.help.clone());
            r.severity = Some(severity);
            r.evidence = v.sample.clone();
            r.fix = Some(format!("See {}", v.help_url));
            r
        });
        // Each passed axe check is its own pass finding, so the category score reflects the
        // real ratio of passes to failures.
        let passes = data.axe_passes.iter().map(|p| {
            let mut r = R::pass("Passed");
            r.rule_id = Some(format!("accessibility/{}", p.id));
            r.title = Some(p.help.clone());
            r.severity = Some(Severity::Warning);
            r
        });
        let out: Vec<R> = violations.chain(passes).collect();
        if out.is_empty() { vec![R::pass("No accessibility violations found by axe-core")] } else { out }
    }
);

#[cfg(test)]
mod tests {
    use crate::Severity;
    use crate::Status::{Fail, Pass, Warn};
    use crate::rules::testkit::page;
    use crate::rules::{AxePass, AxeViolation, RenderedData};

    fn violation(id: &str, impact: &str, nodes: u32) -> AxeViolation {
        AxeViolation {
            id: id.into(),
            impact: Some(impact.into()),
            help: format!("{id} help"),
            description: format!("{id} description"),
            help_url: format!("https://dequeuniversity.com/rules/axe/4.14/{id}"),
            nodes,
            sample: Some("<img src=x>".into()),
        }
    }

    /// FR-RULE-03: 3 axe violations become 3 findings with distinct IDs and their own severities.
    #[test]
    fn one_finding_per_axe_result() {
        let mut p = page("<p>x</p>");
        p.rendered = Some(RenderedData {
            ok: true,
            axe: vec![
                violation("image-alt", "critical", 2),
                violation("label", "moderate", 1),
                violation("region", "minor", 3),
            ],
            axe_passes: vec![AxePass { id: "html-has-lang".into(), help: "html has lang".into() }],
            ..Default::default()
        });
        let r = p.results("accessibility/axe");
        assert_eq!(r.len(), 4);
        let ids: Vec<&str> = r.iter().filter_map(|f| f.rule_id.as_deref()).collect();
        assert_eq!(
            ids,
            ["accessibility/image-alt", "accessibility/label", "accessibility/region", "accessibility/html-has-lang"]
        );
        assert_eq!(r.iter().map(|f| f.status).collect::<Vec<_>>(), [Fail, Fail, Warn, Pass]);
        assert_eq!(r[0].severity, Some(Severity::Error));
        assert_eq!(r[2].severity, Some(Severity::Info));
        assert_eq!(r[0].message, "image-alt description (2 elements, impact: critical)");
        assert_eq!(r[1].message, "label description (1 element, impact: moderate)");
    }

    #[test]
    fn unrendered_and_clean_pages() {
        assert_eq!(page("<p>x</p>").status("accessibility/axe"), Warn);
        let mut clean = page("<p>x</p>");
        clean.rendered = Some(RenderedData { ok: true, ..Default::default() });
        assert_eq!(clean.message("accessibility/axe"), "No accessibility violations found by axe-core");
    }
}
