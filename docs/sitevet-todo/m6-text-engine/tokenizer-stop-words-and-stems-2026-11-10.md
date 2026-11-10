---
id: "tokenizer-stop-words-and-stems-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m6"]
order: 2
---

# Tokenizer, stop words and stems

unicode-segmentation, stop-words, rust-stemmers per language. Hand-counted fixtures for English, Arabic, a German or French page and a Chinese page.

## Requirements

- [ ] FR-TEXT-03 [Must]: Unicode-aware tokenizing, stop words and stemming for the page language, for every language the chosen libraries support. English and Arabic are the tested minimum. Languages without a stop word list still get counts, with stop-word figures marked unavailable.
  Acceptance: English and Arabic fixtures produce correct word counts and stop-word percentages against hand-counted values.

Spec: `docs/spec/06-build-spec.md`, section 8.
