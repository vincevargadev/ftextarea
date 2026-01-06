// ftextarea Service Worker
// Provides offline support by caching all static assets

const CACHE_NAME = 'ftextarea-v1';

// List of files to cache for offline use
const STATIC_ASSETS = [
    '/',
    '/index.html',
    '/style.css',
    '/script.js',
    '/pkg/ftextarea.js',
    '/pkg/ftextarea_bg.wasm',
    '/favicon.ico',
    '/favicon.svg',
    '/apple-touch-icon.png',
    '/site.webmanifest'
];

// Install event: cache all static assets
self.addEventListener('install', event => {
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => {
                console.log('ServiceWorker: Caching static assets');
                return cache.addAll(STATIC_ASSETS);
            })
            .then(() => {
                // Skip waiting to activate immediately
                return self.skipWaiting();
            })
    );
});

// Activate event: clean up old caches
self.addEventListener('activate', event => {
    event.waitUntil(
        caches.keys()
            .then(cacheNames => {
                return Promise.all(
                    cacheNames
                        .filter(name => name !== CACHE_NAME)
                        .map(name => {
                            console.log('ServiceWorker: Deleting old cache:', name);
                            return caches.delete(name);
                        })
                );
            })
            .then(() => {
                // Take control of all clients immediately
                return self.clients.claim();
            })
    );
});

// Fetch event: serve from cache, falling back to network
self.addEventListener('fetch', event => {
    event.respondWith(
        caches.match(event.request)
            .then(cachedResponse => {
                if (cachedResponse) {
                    // Return cached version
                    return cachedResponse;
                }
                
                // Not in cache, fetch from network
                return fetch(event.request)
                    .then(response => {
                        // Don't cache non-successful responses or non-GET requests
                        if (!response || response.status !== 200 || event.request.method !== 'GET') {
                            return response;
                        }
                        
                        // Clone the response (it can only be consumed once)
                        const responseToCache = response.clone();
                        
                        // Add to cache for future offline use
                        caches.open(CACHE_NAME)
                            .then(cache => {
                                cache.put(event.request, responseToCache);
                            });
                        
                        return response;
                    });
            })
            .catch(() => {
                // Network failed and not in cache
                // For HTML requests, we could return an offline page here
                console.log('ServiceWorker: Fetch failed for:', event.request.url);
            })
    );
});

