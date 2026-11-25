#!/bin/bash
# Builds EFFECTOR.app (universal: Apple Silicon + Intel) and EFFECTOR-<version>-macOS.dmg.
# usage: packaging/macos/build_app.sh [--install]
set -euo pipefail
cd "$(dirname "$0")/../.."

VERSION=$(grep -m1 '^version' Cargo.toml | sed -E 's/version = "(.*)"/\1/')
BUILD=$(git rev-list --count HEAD 2>/dev/null || echo 1)
OUT=dist
APP="$OUT/EFFECTOR.app"

rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
export MACOSX_DEPLOYMENT_TARGET=11.0
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/effector" \
  target/aarch64-apple-darwin/release/effector target/x86_64-apple-darwin/release/effector
sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/$BUILD/" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
cp assets/icons/icon.icns "$APP/Contents/Resources/AppIcon.icns"
mkdir -p "$APP/Contents/Resources/en.lproj" "$APP/Contents/Resources/pt-BR.lproj"
touch "$APP/Contents/Resources/en.lproj/InfoPlist.strings" "$APP/Contents/Resources/pt-BR.lproj/InfoPlist.strings"
cp -R assets/fonts "$APP/Contents/Resources/licenses" 2>/dev/null || true
find "$APP/Contents/Resources/licenses" -name '*.ttf' -delete 2>/dev/null || true

# ad-hoc signature (no Apple Developer account, not notarized). Since macOS 15 the right-click → Open bypass is gone:
# users must allow it once in System Settings → Privacy & Security → Open Anyway (see the how-to-open file in the DMG).
codesign --force --deep --sign - "$APP"
echo "Built $APP ($VERSION build $BUILD)"

# disk image with a shortcut to /Applications
DMG="$OUT/EFFECTOR-$VERSION-macOS.dmg"
STAGE="$OUT/dmg"
rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
cp "packaging/macos/COMO ABRIR - HOW TO OPEN.txt" "$STAGE/"
hdiutil create -volname "EFFECTOR $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
rm -rf "$STAGE"
echo "Built $DMG"

if [[ "${1:-}" == "--install" ]]; then
  pkill -x effector 2>/dev/null || true
  sleep 0.5
  rm -rf /Applications/EFFECTOR.app
  cp -R "$APP" /Applications/EFFECTOR.app
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/EFFECTOR.app
  echo "Installed to /Applications/EFFECTOR.app"
fi
