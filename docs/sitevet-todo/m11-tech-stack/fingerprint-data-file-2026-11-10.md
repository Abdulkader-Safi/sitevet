---
id: "fingerprint-data-file-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-23T06:01:30.000Z"
labels: ["m11"]
order: 1
---

# Fingerprint data file

`data/fingerprints.toml` written from scratch. No Wappalyzer data.

## Requirements

- [x] FR-TECH-03 [Must]: Signatures must live in a data file, so adding a vendor is one entry with no engine change.
  Acceptance: a test adds a signature in test data and detects it.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- `data/fingerprints.toml`, 36 vendors written from scratch. A test adds a vendor in test data and detects it.
