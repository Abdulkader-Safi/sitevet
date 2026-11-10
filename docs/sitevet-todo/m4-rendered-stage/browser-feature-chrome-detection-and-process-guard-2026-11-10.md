---
id: "browser-feature-chrome-detection-and-process-guard-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m4"]
order: 1
---

# Browser feature: Chrome detection and process guard

Detect Chrome per OS, `chrome_path` and `SITEVET_CHROME`, guard that kills Chrome on drop, fresh profile per audit, tab pool of 2.

## Requirements

- [ ] FR-RENDER-05 [Must]: If browser is requested but no Chrome binary is found, the audit must fail before crawling, with a message saying how to install one or set its path.
  Acceptance: with `chrome_path` pointing nowhere, the error names the option and the install hint.
- [ ] FR-RENDER-06 [Must]: The browser stage must be a cargo feature. Building without it pulls in no browser dependencies.
  Acceptance: `cargo tree --no-default-features` shows no browser crates.

Spec: `docs/spec/06-build-spec.md`, section 8.
