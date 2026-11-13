---
id: "text-statistics-and-readability-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T13:17:08.000Z"
labels: ["m6"]
order: 3
---

# Text statistics and readability

Statistics, English formulas, neutral fallback, no-word-space fallback.

## Requirements

- [x] FR-TEXT-04 [Must]: Text statistics: total words, unique words and percent, stop words and percent, water percent, average sentence length, characters without spaces.
  Acceptance: fixture values match hand-computed values within 1%.
- [x] FR-TEXT-05 [Must]: Readability for English pages: Flesch reading ease with a label, Flesch-Kincaid grade and Gunning Fog. For every other language, report average sentence length and average word length, and mark the formulas "not available for this language". For scripts without word spaces (Chinese, Japanese, Thai and similar), report character counts and mark word-based metrics not available.
  Acceptance: the English fixture matches reference values. Arabic and Chinese fixtures show the "not available" state, never an English score.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- `content/text-statistics` and `content/readability` ported from the TS package, plus `content/avg-word-length`.
- English fixture: Flesch 83.3, Flesch-Kincaid 2.9, Gunning Fog 4, all hand-computed. Arabic, French and Chinese show "Not available for this language".
