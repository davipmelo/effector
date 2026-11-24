#!/bin/zsh
# Builds EFFECTOR.app (release) and optionally installs it to /Applications.
# usage: scripts/build_app.sh [--install]
set -e
cd "$(dirname "$0")/.."
source ./build.env

VERSION=$(cat VERSION 2>/dev/null || echo "0.1.0")
BUILD=$(git rev-list --count HEAD 2>/dev/null || echo 1)

swift build -c release
BIN=.build/release/Effector
APP=build/EFFECTOR.app

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/Effector"
sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/$BUILD/" scripts/Info.plist > "$APP/Contents/Info.plist"

# Icon
if [[ ! -f build/AppIcon.icns || scripts/make_icon.swift -nt build/AppIcon.icns ]]; then
  rm -rf build/AppIcon.iconset
  swift scripts/make_icon.swift build/AppIcon.iconset
  iconutil -c icns build/AppIcon.iconset -o build/AppIcon.icns
fi
cp build/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

codesign --force --deep --sign - "$APP"
echo "Built $APP ($VERSION build $BUILD)"

if [[ "$1" == "--install" ]]; then
  pkill -x Effector 2>/dev/null || true
  sleep 0.5
  rm -rf /Applications/EFFECTOR.app
  cp -R "$APP" /Applications/EFFECTOR.app
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/EFFECTOR.app
  echo "Installed to /Applications/EFFECTOR.app"
fi
