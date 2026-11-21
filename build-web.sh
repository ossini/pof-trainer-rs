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
sed "s/__BUILD__/$(date +%Y%m%d%H%M%S)/" pwa/sw.js > dist/sw.js

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
    <meta name="theme-color" content="#070b14">
    <link rel="manifest" href="manifest.webmanifest">
    <link rel="apple-touch-icon" href="icon-180.png">
    <link rel="icon" type="image/png" href="icon-192.png">
    <style>html,body{background:#070b14}</style>
    <script>if ('serviceWorker' in navigator) addEventListener('load', () => navigator.serviceWorker.register('sw.js'));</script>
"""
if "apple-mobile-web-app-capable" not in html:
    html = html.replace("</head>", head + "</head>", 1)
html = html.replace("<html>", '<html lang="de">', 1)
open(p, "w", encoding="utf-8").write(html)
EOF

echo "Fertig: $(pwd)/dist"
if [ "${1:-}" = "serve" ]; then
  echo "http://localhost:8090"
  python3 -m http.server 8090 --directory dist
fi
