---
id: "answer-snippet-and-q-a-checks-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m7"]
order: 1
---

# Answer snippet and Q&A checks

## Requirements

- [ ] FR-AEO-01 [Must]: Answer snippet check: a definition or short answer within the first four content blocks, which block, and its length in words.
  Acceptance: a fixture with a 40-word definition in block 2 reports block 2, 40 words.
- [ ] FR-AEO-02 [Must]: Q&A structure: count of question-like H2s, how many have a short answer directly under them, and the coverage ratio.
  Acceptance: a fixture with 4 question H2s, 3 answered, reports 0.75.

Spec: `docs/spec/06-build-spec.md`, section 8.
