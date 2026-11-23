---
id: "cms-framework-and-cdn-detection-2026-11-10"
status: "done"
priority: "medium"
assignee: null
dueDate: null
created: "2026-11-10T08:20:43.000Z"
modified: "2026-11-23T06:01:30.000Z"
labels: ["m11"]
order: 3
---

# CMS, framework and CDN detection

## Requirements

- [x] FR-TECH-02 [Must]: Detect CMS (WordPress, Shopify, Webflow, Wix, Drupal, Joomla, Squarespace, Ghost), frameworks and libraries (React, Next.js, Vue, Nuxt, Angular, Svelte, jQuery, Laravel), and CDN (Cloudflare, Fastly, Akamai, CloudFront, Vercel, Netlify) from markup, script sources and headers.
  Acceptance: recorded responses from a WordPress-on-Cloudflare fixture detect both.

Spec: `docs/spec/06-build-spec.md`, section 8.

## Done

- CMS, framework and CDN signatures. The WordPress-on-Cloudflare fixture site detects both. Laravel is found by its encrypted session cookie.
