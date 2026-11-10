# Third-party files

sitevet is MIT licensed. It bundles one file under another licence.

## axe-core 4.14.0

- File: `crates/sitevet/assets/axe.min.js`, shipped unmodified and compiled into sitevet only with the `browser` feature.
- Licence: Mozilla Public License 2.0, copyright Deque Systems, Inc. Full text in `crates/sitevet/assets/axe-core-LICENSE.txt`, and the licences of axe-core's own bundled code in `crates/sitevet/assets/axe-core-LICENSE-3RD-PARTY.txt`.
- Source: the `axe-core` 4.14.0 package on npm, https://www.npmjs.com/package/axe-core, and https://github.com/dequelabs/axe-core.
- SHA-256 of the bundled file: `20c09fe157a8a34a30e241aaa1fcdade657734f08ab379ecfbeb7d45cc46e878`.

MPL-2.0 applies per file: the axe-core file stays under MPL-2.0 and its source is available at the links above. The rest of sitevet stays MIT.

Rust crate dependencies and their licences are checked in CI with `cargo deny` (see `deny.toml`).
