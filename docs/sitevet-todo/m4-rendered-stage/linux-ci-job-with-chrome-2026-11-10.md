---
id: "linux-ci-job-with-chrome-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T17:43:45.000Z"
labels: ["m4"]
order: 6
---

# Linux CI job with Chrome

Browser tests behind `--features browser` and `SITEVET_TEST_CHROME=1`.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

The `browser` job in `.github/workflows/ci.yml` runs on `ubuntu-latest` with the runner's Chrome and `SITEVET_TEST_CHROME=1`: `cargo test -p sitevet --features browser --lib --test browser --test browser_cancel`. That covers axe and Core Web Vitals, render_crawl, cancel killing Chrome, and the private-network guard for Chrome's requests. Green in run 42866636468.
