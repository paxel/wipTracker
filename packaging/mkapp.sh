#!/usr/bin/env bash
# Assemble the macOS app bundle (WipTracker.app) and a .dmg around it, from an
# already-built `wiptracker` binary. macOS-only: uses hdiutil for the .dmg. The
# bundle is unsigned — first launch needs `xattr -cr` (documented in the README).
#
# Usage:
#   packaging/mkapp.sh <path-to-wiptracker-binary> <version> <out-dir> [arch-label]
#
# The arch label only names the .dmg; it defaults to `uname -m`. Pass `universal` for a
# lipo-merged binary, which is what the release workflow ships.
#
# Writes <out-dir>/WipTracker.app and <out-dir>/wiptracker-<version>-macos-<arch>.dmg.
set -euo pipefail

[ $# -ge 3 ] && [ $# -le 4 ] || { sed -n '2,12p' "$0"; exit 2; }
BIN=$1
VERSION=$2
OUTDIR=$3
ARCH=${4:-$(uname -m)}

[ -f "$BIN" ] || { echo "mkapp: binary not found: $BIN" >&2; exit 1; }
mkdir -p "$OUTDIR"

APP="$OUTDIR/WipTracker.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

install -m755 "$BIN" "$APP/Contents/MacOS/wiptracker"

# Icon: an iconset scaled from the generated PNG, then compiled to .icns.
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# The 1024 pixel render when the generator has written one: the iconset wants 512@2x, and
# upscaling the 512 pixel file for that slot is what it used to do.
ICON_PNG="$ROOT/assets/icon-1024.png"
[ -f "$ICON_PNG" ] || ICON_PNG="$ROOT/assets/icon.png"
if [ -f "$ICON_PNG" ]; then
  ICONSET=$(mktemp -d)/WipTracker.iconset
  mkdir -p "$ICONSET"
  for size in 16 32 64 128 256 512; do
    sips -z $size $size "$ICON_PNG" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z $double $double "$ICON_PNG" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
  done
  iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/WipTracker.icns"
else
  echo "mkapp: no icon in assets/, bundling without one" >&2
fi

# LSUIElement keeps the bar out of the Dock and the app switcher: it is a strip of
# screen furniture, not a window you alt-tab to.
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>WipTracker</string>
  <key>CFBundleDisplayName</key><string>WipTracker</string>
  <key>CFBundleIdentifier</key><string>dev.paxel.wiptracker</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleExecutable</key><string>wiptracker</string>
  <key>CFBundleIconFile</key><string>WipTracker</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

DMG="$OUTDIR/wiptracker-${VERSION}-macos-${ARCH}.dmg"
rm -f "$DMG"
hdiutil create -volname "WipTracker ${VERSION}" -srcfolder "$APP" -ov -format UDZO "$DMG" >/dev/null
echo "mkapp: wrote $APP and $DMG"
