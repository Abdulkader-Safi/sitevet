---
id: "tokenizer-stop-words-and-stems-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T13:17:08.000Z"
labels: ["m6"]
order: 2
---

# Tokenizer, stop words and stems

unicode-segmentation, stop-words, rust-stemmers per language. Hand-counted fixtures for English, Arabic, a German or French page and a Chinese page.

## Requirements

- [x] FR-TEXT-03 [Must]: Unicode-aware tokenizing, stop words and stemming for the page language, for every language the chosen libraries support. English and Arabic are the tested minimum. Languages without a stop word list still get counts, with stop-word figures marked unavailable.
  Acceptance: English and Arabic fixtures produce correct word counts and stop-word percentages against hand-counted values.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- Stop words from the stop-words ISO lists, stems from the Snowball stemmers, for every language they cover. English keeps the TS 201-word list (the ISO one drops words like "page" and "website").
- Hand-counted fixtures: English 20 words and 3 stop words, Arabic 17 and 4, French 13 and 5, Chinese 23 characters.
