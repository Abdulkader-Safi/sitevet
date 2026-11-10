---
id: "language-detection-and-main-content-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m6"]
order: 1
---

# Language detection and main content

`<html lang>` with whatlang fallback and confidence. Main-content extraction without nav, header, footer, aside.

## Requirements

- [ ] FR-TEXT-01 [Must]: Detect the page language from `<html lang>`, falling back to detection from the body text, and report both values.
  Acceptance: an Arabic fixture with no `lang` attribute is detected as Arabic.
- [ ] FR-TEXT-02 [Must]: Main-content extraction must exclude boilerplate (nav, header, footer, aside) from text statistics.
  Acceptance: a fixture whose footer repeats a word 50 times does not show that word in the top keywords.

Spec: `docs/spec/06-build-spec.md`, section 8.
