---
id: "rewrite-keyword-stuffing-on-n-grams-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-13T13:17:08.000Z"
labels: ["m6"]
order: 5
---

# Rewrite keyword-stuffing on n-grams

Same rule ID, new logic.

## Requirements

- [x] FR-TEXT-08 [Must]: `content/keyword-stuffing` must use the n-gram output.
  Acceptance: a stuffed fixture fails, a normal one passes.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- The rule reads the n-gram output in the page's language. Scripts without word spaces pass with "Word-based check not available for this language".
