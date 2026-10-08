#!/usr/bin/env bash
# Vendor PixiJS into assets/. Run from the repository root.
# The script downloads the pinned upstream files to a temp dir and checks
# each sha384. Then it moves the files into assets/. It does not change
# the files. Keep the pins in sync with VENDORED in src/assets.rs and with
# assets/manifest.json.
# Needs curl, openssl, and coreutils (base64 -w0).
#
# To upgrade PixiJS:
# 1. Get each new pin:
#    curl -fsSL --proto '=https' --tlsv1.2 URL | openssl dgst -sha384 -binary | base64 -w0
#    Compare it with the npm tarball of the same version.
# 2. Set VERSION and the pins below.
# 3. Run this script.
# 4. Update PIXI_VERSION and VENDORED (src/assets.rs), assets/manifest.json,
#    and the version in README.md, CLAUDE.md, and src/lib.rs.
# 5. Run all tests (see CLAUDE.md).
set -euo pipefail

VERSION="8.22.0"
BASE="https://cdn.jsdelivr.net/npm/pixi.js@${VERSION}"
OUT="assets"
TMP="$(mktemp -d)"
trap 'rm -rf "${TMP}"' EXIT

# upstream path | served name | pinned sha384
FILES=(
  "dist/pixi.min.js|pixi.min.js|sQhAUuZTvdanRcBHbVCTasnEWH28GGEK87FJGPj/JaqwL/15KWQwKe7whTHdCkwm"
  "dist/packages/unsafe-eval.min.js|unsafe-eval.min.js|BM24TdVWfEguSNFpLGxxbyyXTUrbgHXZZWEciqcl2APKgUzLagtGe2wwuAzmsmsi"
  "LICENSE|PIXI-LICENSE|DW05SCtkW0ljGN5QkX9yxuThaqbPVlFm5ibuTACn9yRGki4gBN5kdXpyb5L60g2t"
)

for entry in "${FILES[@]}"; do
  IFS='|' read -r upstream name pin <<<"${entry}"
  curl -fsSL --proto '=https' --tlsv1.2 "${BASE}/${upstream}" -o "${TMP}/${name}"
  actual="$(openssl dgst -sha384 -binary "${TMP}/${name}" | base64 -w0)"
  if [[ "${actual}" != "${pin}" ]]; then
    echo "sha384 mismatch: ${upstream}" >&2
    exit 1
  fi
done

for entry in "${FILES[@]}"; do
  IFS='|' read -r _ name _ <<<"${entry}"
  mv "${TMP}/${name}" "${OUT}/${name}"
done
echo "vendored pixi.js@${VERSION}"
