#!/usr/bin/env bash
# Cross-compile a Windows .exe from macOS and zip it for sharing.
#
# Needs two one-time installs:
#     brew install mingw-w64
#     rustup target add x86_64-pc-windows-gnu
#
#   ./package-windows.sh          -> dist/GearMaster-Windows.zip
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKG="$ROOT/packaging"
DIST="$ROOT/dist"
ZIP="$DIST/GearMaster-Windows.zip"
BIN=gearmaster-gui
TARGET=x86_64-pc-windows-gnu

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }

missing=0
if ! command -v x86_64-w64-mingw32-gcc >/dev/null; then
  echo "  MISSING: the mingw-w64 linker."
  echo "           brew install mingw-w64"
  missing=1
fi
if ! rustup target list --installed | grep -q "^$TARGET$"; then
  echo "  MISSING: the Rust target."
  echo "           rustup target add $TARGET"
  missing=1
fi
if [ "$missing" -eq 1 ]; then
  echo
  echo "  Install those, then re-run. (Or build on a Windows machine with"
  echo "  plain \`cargo build --release -p $BIN\` — no cross-compiling needed.)"
  exit 1
fi

say "Building $TARGET"
cargo build --release --target "$TARGET" -p "$BIN"

mkdir -p "$DIST"
rm -f "$ZIP"
STAGE="$DIST/.win"
rm -rf "$STAGE"
mkdir -p "$STAGE"

cp "$ROOT/target/$TARGET/release/$BIN.exe" "$STAGE/GearMaster.exe"
cp "$PKG/READ-ME-FIRST-WINDOWS.txt" "$STAGE/READ-ME-FIRST.txt"

say "Zipping"
( cd "$STAGE" && zip -q -r "$ZIP" GearMaster.exe READ-ME-FIRST.txt )
rm -rf "$STAGE"

echo
say "Done"
echo "  $ZIP  ($(du -h "$ZIP" | cut -f1 | tr -d ' '))"
echo
echo "  Windows SmartScreen will warn on first launch — READ-ME-FIRST.txt"
echo "  covers it."
