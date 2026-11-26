#!/bin/bash
# Downloads the subject detection model (IS-Net general use, Apache-2.0, ONNX) into assets/models.
# Used by the release / CI workflows and for local builds. Safe to run again (skips when present and valid).
# Tries the copy in this project's releases first, then the rembg project's release.
set -euo pipefail
cd "$(dirname "$0")/.."
URLS=(
  "https://github.com/davipmelo/effector/releases/download/subject-model-v1/subject_isnet.onnx"
  "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx"
)
SHA="60920e99c45464f2ba57bee2ad08c919a52bbf852739e96947fbb4358c0d964a"
OUT=assets/models/subject_isnet.onnx
mkdir -p assets/models
check() { [ -f "$OUT" ] && [ "$( (sha256sum "$OUT" 2>/dev/null || shasum -a 256 "$OUT") | cut -d' ' -f1)" = "$SHA" ]; }
if check; then echo "subject model ok"; exit 0; fi
for u in "${URLS[@]}"; do
  if curl -fsSL -o "$OUT" "$u" && check; then echo "subject model downloaded"; exit 0; fi
done
rm -f "$OUT"
echo "subject model download failed"
exit 1
