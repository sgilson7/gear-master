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

  /* The soundtrack. SoundCloud's own widget, streamed from SoundCloud - the
     game itself has no audio engine and nothing is bundled. It sits over a
     corner of the letterbox and can be folded away if it is in the way. */
  #music {
    position: fixed; right: 12px; bottom: 12px; z-index: 20;
    width: 360px; border: 1px solid #3a3a52; border-radius: 6px;
    background: #12121c; overflow: hidden;
  }
  #music.folded #sc { display: none; }
  #music header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 6px 10px; font-size: 12px; letter-spacing: 1px; color: #8a8aa4;
    cursor: pointer; user-select: none;
  }
  #music header:hover { color: #f0c85a; }
  #sc { display: block; width: 100%; height: 120px; border: 0; }
</style>
</head>
<body>
  <div id="loading">
    <div class="title">GEAR MASTER</div>
    <div class="sub">loading…</div>
  </div>
  <canvas id="glcanvas" tabindex="1"></canvas>

  <div id="music">
    <header id="music-toggle"><span>SOUNDTRACK</span><span id="music-caret">fold</span></header>
    <iframe id="sc" allow="autoplay" scrolling="no" frameborder="no"
      src="https://w.soundcloud.com/player/?url=https%3A%2F%2Fsoundcloud.com%2Fleefy5%2Fsets%2Fcountless-days-in-ableton&color=%23f0c85a&auto_play=false&hide_related=true&show_comments=false&show_user=true&show_reposts=false&show_teaser=false&visual=false"></iframe>
  </div>

  <script src="https://w.soundcloud.com/player/api.js"></script>
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

    // ---- soundtrack -------------------------------------------------
    // Browsers refuse to start audio before the visitor has interacted with
    // the page, so the playlist starts on the first click or keypress rather
    // than on load. Once it is going, the end of the set seeks back to the
    // first track, which is what makes it repeat.
    (function () {
      if (typeof SC === "undefined") return;
      const widget = SC.Widget(document.getElementById("sc"));
      let started = false;

      widget.bind(SC.Widget.Events.READY, function () {
        widget.setVolume(60);
        widget.getSounds(function (sounds) {
          var last = sounds.length - 1;
          widget.bind(SC.Widget.Events.FINISH, function () {
            // FINISH fires at the end of EVERY track, not of the whole set.
            // Seeking to the top on each one is why the first song played on
            // a loop forever. Only the last track wraps; the widget advances
            // the rest by itself.
            widget.getCurrentSoundIndex(function (i) {
              if (i >= last) {
                widget.skip(0);
                widget.play();
              }
            });
          });
        });
      });

      function start() {
        if (started) return;
        started = true;
        widget.play();
        window.removeEventListener("pointerdown", start, true);
        window.removeEventListener("keydown", start, true);
      }
      // Capture phase: the canvas swallows its own events otherwise.
      window.addEventListener("pointerdown", start, true);
      window.addEventListener("keydown", start, true);

      // Folding hides the player without stopping it.
      const panel = document.getElementById("music");
      document.getElementById("music-toggle").addEventListener("click", function () {
        panel.classList.toggle("folded");
        document.getElementById("music-caret").textContent =
          panel.classList.contains("folded") ? "show" : "fold";
      });
    })();
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
