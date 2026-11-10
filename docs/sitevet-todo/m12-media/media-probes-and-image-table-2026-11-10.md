---
id: "media-probes-and-image-table-2026-11-10"
status: "todo"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-10T08:20:43.000Z"
labels: ["m12"]
order: 1
---

# Media probes and image table

`media` feature, ranged GET with imagesize, request cap.

## Requirements

- [ ] FR-MEDIA-01 [Must]: With a `media` option on (default off), fetch each image with HEAD or a ranged GET, capped by `media_max_requests` (default 200), and report a per-image table: URL, pixel dimensions, transfer size, format, optimisation verdict.
  Acceptance: with the option off, no image requests are made. With it on, the fixture table lists dimensions and sizes.

Spec: `docs/spec/06-build-spec.md`, section 8.
