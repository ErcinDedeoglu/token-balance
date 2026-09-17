#!/usr/bin/env bash
# Same comparison as .github/workflows/release.yml: tag without leading v vs workspace version.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
tag="${1:?tag}"
tag="${tag#v}"
ver=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)
test -n "$ver"
test "$tag" = "$ver"
