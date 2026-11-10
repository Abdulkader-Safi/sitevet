---
id: "redirect-map-and-sitemap-reconciliation-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m8"]
order: 4
---

# Redirect map and sitemap reconciliation

## Requirements

- [ ] FR-SITE-05 [Must]: Site-wide redirect map: every internal link that lands on a redirect and where it ends.
  Acceptance: a link to `/old` that redirects to `/new` appears with both.
- [ ] FR-SITE-06 [Must]: Sitemap versus crawl reconciliation in both directions: in the sitemap but not crawlable, and crawlable but not in the sitemap.
  Acceptance: the fixture site produces one entry in each list.

Spec: `docs/spec/06-build-spec.md`, section 8.
