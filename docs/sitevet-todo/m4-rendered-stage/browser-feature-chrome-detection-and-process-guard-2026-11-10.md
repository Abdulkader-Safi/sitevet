---
id: "browser-feature-chrome-detection-and-process-guard-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T14:46:04.000Z"
labels: ["m4"]
order: 1
---

# Browser feature: Chrome detection and process guard

Detect Chrome per OS, `chrome_path` and `SITEVET_CHROME`, guard that kills Chrome on drop, fresh profile per audit, tab pool of 2.

## Requirements

- [x] FR-RENDER-05 [Must]: If browser is requested but no Chrome binary is found, the audit must fail before crawling, with a message saying how to install one or set its path.
  Acceptance: with `chrome_path` pointing nowhere, the error names the option and the install hint.
- [x] FR-RENDER-06 [Must]: The browser stage must be a cargo feature. Building without it pulls in no browser dependencies.
  Acceptance: `cargo tree --no-default-features` shows no browser crates.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Result

`crates/sitevet/src/render_stage/browser.rs`, compiled only with the `browser` feature (chromiumoxide 0.9 as an optional dependency; the CLI turns the feature on). Chrome is found from `chrome_path`, then `SITEVET_CHROME`, then `PATH` names and the usual install folders per OS. Each audit launches it with a fresh profile in the temp folder (removed afterwards), 1280x800, new headless mode, downloads denied, the sandbox on (off only when running as root on Linux, with a warning). 2 tabs at a time, 30 s per page, one restart if Chrome dies, then the stage stops with a reason. Chrome is launched before the crawl and killed hard right after rendering; dropping the audit kills it too (`kill_on_drop`).

FR-RENDER-05: `chrome_path` pointing nowhere fails before any request with "no Chrome or Chromium found (tried /nowhere/chrome (chrome_path)). Install Google Chrome or Chromium, or set `chrome_path` (or SITEVET_CHROME) to its binary". FR-RENDER-06: the `lean-library` CI job fails if the default library's tree has chromiumoxide.

The FR-API-03 Chromium half: `tests/browser_cancel.rs` cancels mid-crawl and counts Chrome processes by profile name; none are left within 1 s.
