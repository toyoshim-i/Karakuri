// Karakuri Web Service Worker for 100% offline standalone operation (venue-ready)
const CACHE_NAME = 'karakuri-v1';

const CORE_ASSETS = [
  './',
  './index.html',
  './manifest.json',
  './icon.svg',
  './projector.html',
  './receiver.html',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(CORE_ASSETS).catch((err) => {
        console.warn('[SW] Pre-caching core assets warning:', err);
      });
    }).then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) => {
      return Promise.all(
        keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key))
      );
    }).then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET') {
    return;
  }

  const url = new URL(req.url);

  // Skip non-http(s) schemes and third-party APIs
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    return;
  }

  // Bypass local log relay, Cloudflare Turnstile, and external LLM/cloud APIs
  if (url.pathname === '/log' ||
      url.hostname.includes('cloudflare.com') ||
      url.hostname.includes('workers.dev') ||
      url.hostname.includes('twitter.com') ||
      url.hostname.includes('api.openai.com')) {
    return;
  }

  // Same-origin assets: Stale-While-Revalidate with Cache-First fallback for offline resilience
  event.respondWith(
    caches.match(req).then((cached) => {
      const fetchPromise = fetch(req).then((networkRes) => {
        if (networkRes && networkRes.status === 200) {
          const resClone = networkRes.clone();
          caches.open(CACHE_NAME).then((cache) => {
            cache.put(req, resClone);
          });
        }
        return networkRes;
      }).catch((err) => {
        if (cached) {
          return cached;
        }
        throw err;
      });

      return cached || fetchPromise;
    })
  );
});
