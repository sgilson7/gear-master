#!/usr/bin/env bash
# Build a double-clickable GearMaster.app for macOS and zip it for sharing.
#
# Produces a *universal* binary (Apple Silicon + Intel) so it runs on any Mac
# from the last decade, wraps it in a proper .app bundle so double-clicking
# launches the game instead of a Terminal window, ad-hoc signs it, and zips it
# with `ditto` (which preserves the bundle's metadata — plain `zip` can mangle
# it).
#
#   ./package-macos.sh            -> dist/GearMaster-macOS.zip
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKG="$ROOT/packaging"
DIST="$ROOT/dist"
APP="$DIST/GearMaster.app"
ZIP="$DIST/GearMaster-macOS.zip"
BIN=gearmaster-gui
VERSION="0.1.0"

ARM_TARGET=aarch64-apple-darwin
INTEL_TARGET=x86_64-apple-darwin

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }

# ---------------------------------------------------------------- build

say "Building release binaries"
cargo build --release --target "$ARM_TARGET" -p "$BIN"

ARM_BIN="$ROOT/target/$ARM_TARGET/release/$BIN"
INTEL_BIN="$ROOT/target/$INTEL_TARGET/release/$BIN"
SLICES=("$ARM_BIN")

# The Intel slice is optional: without it the app still runs everywhere via
# Rosetta-free arm64, it just won't launch on an Intel Mac.
if rustup target list --installed | grep -q "^$INTEL_TARGET$"; then
  cargo build --release --target "$INTEL_TARGET" -p "$BIN"
  SLICES+=("$INTEL_BIN")
else
  echo
  echo "  NOTE: $INTEL_TARGET is not installed, so this build is Apple-Silicon only."
  echo "        Intel Mac friends won't be able to run it. To include them:"
  echo "            rustup target add $INTEL_TARGET"
  echo
fi

# ------------------------------------------------------------ .app bundle

say "Assembling $(basename "$APP")"
rm -rf "$APP" "$ZIP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

if [ "${#SLICES[@]}" -gt 1 ]; then
  lipo -create -output "$APP/Contents/MacOS/GearMaster" "${SLICES[@]}"
else
  cp "${SLICES[0]}" "$APP/Contents/MacOS/GearMaster"
fi
chmod +x "$APP/Contents/MacOS/GearMaster"

# Generate the icon on first run, then reuse it.
if [ ! -f "$PKG/AppIcon.icns" ] && command -v python3 >/dev/null; then
  python3 "$PKG/make-icon.py" || true
fi
[ -f "$PKG/AppIcon.icns" ] && cp "$PKG/AppIcon.icns" "$APP/Contents/Resources/"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>              <string>GearMaster</string>
  <key>CFBundleDisplayName</key>       <string>Gear Master</string>
  <key>CFBundleExecutable</key>        <string>GearMaster</string>
  <key>CFBundleIdentifier</key>        <string>com.gearmaster.prototype</string>
  <key>CFBundleVersion</key>           <string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundlePackageType</key>       <string>APPL</string>
  <key>CFBundleIconFile</key>          <string>AppIcon</string>
  <key>LSMinimumSystemVersion</key>    <string>11.0</string>
  <key>NSHighResolutionCapable</key>   <true/>
  <key>LSApplicationCategoryType</key> <string>public.app-category.games</string>
</dict>
</plist>
PLIST

# Ad-hoc signature. This does NOT get past Gatekeeper on a downloaded app —
# only a paid Developer ID plus notarisation does that — but it keeps macOS
# from refusing to run the arm64 slice at all, and it stops the bundle looking
# tampered with.
say "Ad-hoc signing"
codesign --force --deep --sign - "$APP" 2>/dev/null || \
  echo "  (codesign unavailable; continuing unsigned)"

cp "$PKG/READ-ME-FIRST.txt" "$DIST/READ-ME-FIRST.txt"

# ------------------------------------------------------------------- zip

say "Zipping"
# `ditto` is the macOS-native archiver; it keeps the bundle structure and the
# executable bit intact where plain `zip` can lose them.
( cd "$DIST" && ditto -c -k --sequesterRsrc --keepParent GearMaster.app "$(basename "$ZIP")" )

SIZE=$(du -h "$ZIP" | cut -f1 | tr -d ' ')
ARCHES=$(lipo -archs "$APP/Contents/MacOS/GearMaster" 2>/dev/null || echo "unknown")

echo
say "Done"
echo "  $ZIP  ($SIZE, $ARCHES)"
echo "  $DIST/READ-ME-FIRST.txt"
echo
echo "  Send BOTH to your friends. Discord's free upload limit is 10 MB —"
echo "  this is well under it."
echo
echo "  Anyone who downloads it will hit a Gatekeeper warning, because the app"
echo "  isn't signed with a paid Apple Developer ID. READ-ME-FIRST.txt walks"
echo "  them through it in two clicks."
