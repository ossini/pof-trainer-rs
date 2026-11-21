#!/usr/bin/env bash
# Web/PWA build: Rust -> WASM, plus manifest, icons and service worker so the
# app can be added to the iPhone home screen and works offline.
#
#   ./build-web.sh            -> dist/ (upload anywhere with HTTPS)
#   ./build-web.sh serve      -> also serves dist/ on http://localhost:8090
set -euo pipefail
cd "$(dirname "$0")"

dx bundle --platform web --release

OUT=$(find target/dx -type d -path '*release/web/public' | head -n1)
[ -n "$OUT" ] || { echo "Build-Ausgabe nicht gefunden (target/dx/*/release/web/public)"; exit 1; }

rm -rf dist
cp -r "$OUT" dist
cp pwa/manifest.webmanifest pwa/icon-180.png pwa/icon-192.png pwa/icon-512.png dist/
python3 - dist/index.html <<'EOF'
import sys
p = sys.argv[1]
html = open(p, encoding="utf-8").read()
head = """
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="apple-mobile-web-app-title" content="PoF">
    <meta name="theme-color" content="#111317">
    <link rel="manifest" href="manifest.webmanifest">
    <link rel="apple-touch-icon" href="icon-180.png">
    <link rel="icon" type="image/png" href="icon-192.png">
    <style>html,body{background:#111317}</style>
    <script>
    // Offline cache + updates: a new version takes over by itself right after
    // start; later on, a button offers the reload (never mid-question).
    if ('serviceWorker' in navigator) {
      const t0 = Date.now();
      const hadController = !!navigator.serviceWorker.controller;
      let done = false;
      navigator.serviceWorker.addEventListener('controllerchange', () => {
        if (!hadController || done) return;
        done = true;
        if (Date.now() - t0 < 15000) { location.reload(); return; }
        const b = document.createElement('button');
        b.textContent = 'Neue Version – tippen zum Laden';
        b.style.cssText = 'position:fixed;left:50%;transform:translateX(-50%);top:calc(env(safe-area-inset-top) + 8px);z-index:99;padding:10px 16px;border:0;border-radius:999px;background:#6aa1ff;color:#0b1220;font:600 14px -apple-system,system-ui,sans-serif;box-shadow:0 4px 16px rgba(0,0,0,.3)';
        b.onclick = () => location.reload();
        document.body.appendChild(b);
      });
      addEventListener('load', () => navigator.serviceWorker.register('sw.js').then((reg) => {
        document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') reg.update(); });
      }));
    }
    </script>
"""
if "apple-mobile-web-app-capable" not in html:
    html = html.replace("</head>", head + "</head>", 1)
html = html.replace("<html>", '<html lang="de">', 1)
open(p, "w", encoding="utf-8").write(html)
EOF

# Service worker with the list of every file to keep offline
python3 - dist pwa/sw.js <<'EOF'
import json, sys, time
from pathlib import Path
dist, src = Path(sys.argv[1]), Path(sys.argv[2])
files = ["./"] + sorted(
    "./" + p.relative_to(dist).as_posix()
    for p in dist.rglob("*")
    if p.is_file() and p.name not in ("sw.js", "404.html") and not p.name.startswith(".")
)
sw = src.read_text(encoding="utf-8")
sw = sw.replace("__BUILD__", time.strftime("%Y%m%d%H%M%S")).replace("__FILES__", json.dumps(files, indent=1))
(dist / "sw.js").write_text(sw, encoding="utf-8")
print(f"Offline-Cache: {len(files)} Dateien")
EOF

echo "Fertig: $(pwd)/dist"
if [ "${1:-}" = "serve" ]; then
  echo "http://localhost:8090"
  python3 -m http.server 8090 --directory dist
fi
