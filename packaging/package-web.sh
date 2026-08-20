#!/usr/bin/env bash
# Build the browser version and bundle it for hosting.
#
# Produces dist/web/ (servable as-is) and dist/GearMaster-Web.zip, which is the
# exact shape itch.io wants for an HTML5 game: index.html at the archive root.
#
#   ./package-web.sh              -> dist/web/, dist/GearMaster-Web.zip
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
WEB="$DIST/web"
ZIP="$DIST/GearMaster-Web.zip"
BIN=gearmaster-gui
TARGET=wasm32-unknown-unknown

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }

if ! rustup target list --installed | grep -q "^$TARGET$"; then
  echo "  MISSING: rustup target add $TARGET"
  exit 1
fi

say "Building $TARGET"
cargo build --release --target "$TARGET" -p "$BIN"

# macroquad ships the JS shim that boots the wasm module inside its own crate,
# so take it from the exact version the lockfile pinned rather than a CDN.
MQ_VERSION=$(grep -A1 'name = "macroquad"' "$ROOT/Cargo.lock" | grep version | head -1 | cut -d'"' -f2)
REGISTRY=$(find "$HOME/.cargo/registry/src" -maxdepth 1 -type d -name 'index.crates.io-*' | head -1)
JS_BUNDLE="$REGISTRY/macroquad-$MQ_VERSION/js/mq_js_bundle.js"
if [ ! -f "$JS_BUNDLE" ]; then
  echo "  Couldn't find mq_js_bundle.js for macroquad $MQ_VERSION at:"
  echo "    $JS_BUNDLE"
  echo "  Run \`cargo fetch\` and try again."
  exit 1
fi

say "Assembling $WEB"
rm -rf "$WEB" "$ZIP"
mkdir -p "$WEB"
cp "$ROOT/target/$TARGET/release/$BIN.wasm" "$WEB/gearmaster.wasm"
cp "$JS_BUNDLE" "$WEB/mq_js_bundle.js"

cat > "$WEB/index.html" <<'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, user-scalable=no">
<title>Gear Master</title>
<style>
  html, body {
    margin: 0; padding: 0; height: 100%; overflow: hidden;
    background: #08080c; color: #d8dae8;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  /* The canvas fills the window at 1:1 CSS pixels. The game letterboxes its
     own fixed 1600x980 layout inside whatever it gets, so no CSS transform is
     involved — which matters, because a transform would break mouse mapping. */
  #glcanvas { display: block; width: 100vw; height: 100vh; outline: none; }
  #loading {
    position: fixed; inset: 0; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 14px;
    background: #08080c; z-index: 10;
  }
  #loading .title { font-size: 26px; letter-spacing: 3px; color: #f0c85a; }
  #loading .sub { font-size: 13px; color: #6a6a80; }
</style>
</head>
<body>
  <div id="loading">
    <div class="title">GEAR MASTER</div>
    <div class="sub">loading…</div>
  </div>
  <canvas id="glcanvas" tabindex="1"></canvas>
  <script src="mq_js_bundle.js"></script>
  <script>
    // The shim swaps to the canvas once the module is up; give it a beat, then
    // clear the splash and hand it keyboard focus.
    load("gearmaster.wasm");
    (function waitForStart() {
      const canvas = document.getElementById("glcanvas");
      if (canvas.width > 0 && canvas.height > 0) {
        document.getElementById("loading").style.display = "none";
        canvas.focus();
      } else {
        setTimeout(waitForStart, 100);
      }
    })();
    // Right-click rotates pieces, so don't let the browser menu eat it.
    document.getElementById("glcanvas")
      .addEventListener("contextmenu", e => e.preventDefault());
  </script>
</body>
</html>
HTML

say "Zipping"
( cd "$WEB" && zip -q -r "$ZIP" index.html gearmaster.wasm mq_js_bundle.js )

WASM_SIZE=$(du -h "$WEB/gearmaster.wasm" | cut -f1 | tr -d ' ')
ZIP_SIZE=$(du -h "$ZIP" | cut -f1 | tr -d ' ')

echo
say "Done"
echo "  $WEB/            ($WASM_SIZE wasm — serve this folder)"
echo "  $ZIP  ($ZIP_SIZE — upload this to itch.io)"
echo
echo "  Try it locally first:   make serve"
echo
echo "  To share a link:"
echo "    itch.io       New project > Kind: HTML > upload the zip >"
echo "                  tick \"This file will be played in the browser\"."
echo "                  Set the viewport to 1600x980. Free, no account needed"
echo "                  by your friends, and you can keep the page unlisted."
echo "    GitHub Pages  Commit dist/web/ to a repo, enable Pages for that"
echo "                  folder. Also free."
echo
echo "  Then paste the link in Discord. No download, no security warnings,"
echo "  and it works on Windows and Linux too."
