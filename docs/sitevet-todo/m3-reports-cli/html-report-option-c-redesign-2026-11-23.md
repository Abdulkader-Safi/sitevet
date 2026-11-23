---
id: "html-report-option-c-redesign-2026-11-23"
status: "review"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-23T06:41:26.000Z"
modified: "2026-11-23T06:41:26.000Z"
labels: ["m3", "report", "design", "needs-review"]
order: 10
---

# HTML report: option C redesign

Rebuild the exported HTML report as design option C from the Figma file (frames `12:309` dark and `14:1230` light).

Done:

- [x] Three panes: categories, pages (lowest score first), page detail split into tabs
- [x] Top bar with the SITEVET wordmark (inline SVG), view tabs and the theme switch
- [x] KPI strip with totals, pages, change since the baseline and duration
- [x] Search for pages and categories, category filter, sort by path
- [x] Light and dark tokens from the Figma variables, theme remembered in local storage
- [x] Overview, Site-wide and Changes views
- [x] Reads without JavaScript: radio inputs for views and tabs, the URL fragment picks the page
- [x] Stacks into one column on narrow screens (checked at 375 px)
- [x] Stage 05 spec updated

> [!warning] Needs Safi
> Visual check of a real report in both themes. Stage 06 still lists only `report.css` under `assets/` and needs regenerating from 05.
