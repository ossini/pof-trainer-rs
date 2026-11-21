// Offline cache for the home-screen app. The progress itself is NOT here –
// it lives in localStorage on the device and is never sent anywhere.
//
// Every file of the build is downloaded at install time (list filled in by
// build-web.sh), and everything is served from the cache afterwards, so the
// app works in flight mode right after the first visit and a captive-portal
// page (plane / hotel Wi-Fi) can never replace it. Updates arrive as a new
// sw.js; it only takes over once all its files were downloaded completely.
const CACHE = 'pof-trainer-__BUILD__';
const FILES = __FILES__;

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE)
      .then((c) => c.addAll(FILES.map((f) => new Request(f, { cache: 'reload' }))))
      .then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET' || new URL(req.url).origin !== location.origin) return;

  event.respondWith((async () => {
    const cache = await caches.open(CACHE);
    // App start / reload: always the cached page of this version
    if (req.mode === 'navigate') {
      const page = await cache.match('./index.html');
      if (page) return page;
    }
    const hit = await cache.match(req, { ignoreSearch: true });
    if (hit) return hit;
    try {
      return await fetch(req);
    } catch (e) {
      return (await cache.match('./index.html')) || Response.error();
    }
  })());
});

// The app asks whether everything is stored for offline use
self.addEventListener('message', (event) => {
  if (event.data !== 'offline-status') return;
  event.waitUntil((async () => {
    const cache = await caches.open(CACHE);
    const missing = [];
    for (const f of FILES) if (!(await cache.match(f))) missing.push(f);
    event.source.postMessage({ type: 'offline-status', ready: missing.length === 0, files: FILES.length, missing: missing.length });
  })());
});
