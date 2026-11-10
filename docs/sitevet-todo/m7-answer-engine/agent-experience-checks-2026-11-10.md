---
id: "agent-experience-checks-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m7"]
order: 4
---

# Agent experience checks

AI crawler access in robots.txt, `llms.txt`, content present without JavaScript.

## Requirements

- [ ] FR-AEO-05 [Must]: Agent experience: whether robots.txt allows each AI crawler (GPTBot, ClaudeBot, PerplexityBot, CCBot, Google-Extended). Whether `llms.txt` exists and parses. Whether the main content is present in the raw HTML without JavaScript.
  Acceptance: a robots fixture blocking GPTBot reports GPTBot blocked and the others allowed.

Spec: `docs/spec/06-build-spec.md`, section 8.
