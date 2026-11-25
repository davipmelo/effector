#!/bin/bash
# Downloads the depth model (Depth Anything V2 Small, Apache-2.0, converted to f16) into assets/models.
# Used by the release workflow and for local builds. Safe to run again (skips when present and valid).
set -euo pipefail
cd "$(dirname "$0")/.."
URL="https://github.com/davipmelo/effector/releases/download/depth-model-v1/depth_anything_v2_vits_f16.safetensors"
SHA="73e34d35b475bc075ac0b061671e5780800ed734c7eba2f702755f756b60b171"
OUT=assets/models/depth_anything_v2_vits_f16.safetensors
mkdir -p assets/models
check() { [ -f "$OUT" ] && [ "$( (sha256sum "$OUT" 2>/dev/null || shasum -a 256 "$OUT") | cut -d' ' -f1)" = "$SHA" ]; }
if check; then echo "depth model ok"; exit 0; fi
curl -fsSL -o "$OUT" "$URL"
check || { echo "depth model checksum mismatch"; rm -f "$OUT"; exit 1; }
echo "depth model downloaded"
