---
id: "rendered-timings-and-inp-state-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m10"]
order: 1
---

# Rendered timings and INP state

## Requirements

- [ ] FR-PERF-01 [Must]: With the rendered stage, report total load time, DNS, TCP, response time, DOM interactive, DOM content loaded, DOM complete and FCP.
  Acceptance: all values are present and non-negative for a rendered fixture.
- [ ] FR-PERF-02 [Must]: INP reported with an explicit "not measurable without interaction" state when no interaction happened, never as zero or empty.
  Acceptance: a default rendered audit shows that state.

Spec: `docs/spec/06-build-spec.md`, section 8.
