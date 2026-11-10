#!/usr/bin/env bash
# Downloads axe-core 4.14.0 unmodified (MPL-2.0) into spikes/assets.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p assets
curl -fsSL https://registry.npmjs.org/axe-core/-/axe-core-4.14.0.tgz | tar -xz -C assets package/axe.min.js package/LICENSE
mv assets/package/axe.min.js assets/package/LICENSE assets/ && rmdir assets/package
