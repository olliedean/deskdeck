const CACHE_NAME = "deskdeck-shell-v5";
const SHELL = ["/", "/index.html", "/styles.css", "/app.js", "/manifest.webmanifest", "/icon.svg"];

self.addEventListener("install", event => {
    event.waitUntil(caches.open(CACHE_NAME).then(cache => cache.addAll(SHELL)));
    self.skipWaiting();
});

self.addEventListener("activate", event => {
    event.waitUntil(
        caches.keys().then(keys => Promise.all(
            keys.filter(key => key !== CACHE_NAME).map(key => caches.delete(key)),
        )),
    );
    self.clients.claim();
});

self.addEventListener("fetch", event => {
    const requestUrl = new URL(event.request.url);
    if (event.request.method !== "GET" || requestUrl.origin !== self.location.origin) return;
    if (requestUrl.pathname === "/deck" || requestUrl.pathname.startsWith("/deck/") || requestUrl.pathname === "/ws") return;

    if (event.request.mode === "navigate") {
        event.respondWith(fetch(event.request).catch(() => caches.match("/index.html")));
        return;
    }

    event.respondWith(caches.match(event.request).then(cached => cached || fetch(event.request)));
});
