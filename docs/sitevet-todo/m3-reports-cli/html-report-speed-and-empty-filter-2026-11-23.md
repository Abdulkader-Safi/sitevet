---
id: "html-report-speed-and-empty-filter-2026-11-23"
status: "review"
priority: "high"
assignee: null
dueDate: null
created: "2026-11-23T10:40:00.000Z"
modified: "2026-11-23T10:40:00.000Z"
labels: ["m3", "report", "performance", "needs-review"]
order: 11
---

# HTML report: speed and empty category filter

A 100-page report was 12.7 MB with 279,000 elements. Selecting a page took 1.1 s and the theme switch took 1 to 2 s, because every page's full detail sat in the document at once.

Done:

- [x] Each page's detail is rendered once in Rust (`templates/report_page.html`) and stored as HTML in an inline JSON block (`#pages-data`)
- [x] The script adds only the open page to the document: about 4,000 elements on a 100-page report, page select 19 ms, theme switch 6 ms
- [x] The JSON escapes `</` and `<!--`, so page text cannot close the script element (checked in the report tests)
- [x] Macros moved to `templates/report_macros.html`, shared by the page and site views
- [x] When the category filter or search hides the open page, the detail moves to the first page still listed
- [x] When no page has issues in a category, the list says so and links to its site-wide findings (Crawlability click depth and orphan pages, for example)
- [x] Stage 05 updated: the detail pane needs JavaScript, the other views still read without it

Related: [[HTML report: option C redesign]]

> [!warning] Needs Safi
> Visual check on a large report. Stage 06 still needs regenerating from 05.
