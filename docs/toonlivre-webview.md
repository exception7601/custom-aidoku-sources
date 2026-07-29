# ToonLivre WebView and API Decryption Strategy

This document details the architecture, mechanics, and implementation of the WebView chapter loading and API decryption strategy for ToonLivre.

## Architecture Overview

ToonLivre protects its chapter pages endpoint using a dynamic decryption layer.
The client-side JavaScript on `toonlivre.net` fetches the chapter data, decrypts it using the Rabbit cipher, and caches the plain JSON in `sessionStorage`.
The key format for the session storage cache is:
`toonlivre_chapter_cache_v1:{mangaId}:{chapterId}`

To bypass manual replication of the dynamic decryption logic, we load the chapter page inside a background `WebView`.
Once the page loads and hydrates, we query `sessionStorage` via JavaScript evaluation to retrieve the decrypted page list.

---

## WebView Initialization and Challenges

Several browser-level and security checks on `toonlivre.net` prevent a default background `WebView` from executing successfully:

### 1. Viewport Visibility and Layout Constraints
Background WebViews on iOS (WKWebView) load with a visibility state of `"hidden"` and dimensions of `0x0`.
ToonLivre's React application suspends rendering or fails hydration under these conditions.

**Solution**:
We inject a `WebViewUserScript` at document start to mock the DOM properties before any page script executes.

This script patches:
- `document.visibilityState` to `"visible"`
- `document.hidden` to `false`
- `window.innerWidth` and `window.innerHeight` to `1280x1920`
- `window.outerWidth` and `window.outerHeight` to `1280x1920`

### 2. Session and Cookie Requirements
ToonLivre requires a session cookie (`toon_v`) on the initial document request.
If the cookie is absent, Next.js server-side rendering renders a "Manga not found" error page instead of the chapter reader.

**Solution**:
We pass the manual `Cookie` header containing a generated session ID on all WebView requests.
Subsequent chapter requests skip the homepage load and navigate directly using the cached session cookie.

### 3. Language Routing Obstacles
If the device locale is set to a non-Portuguese language, the WebView makes subresource API requests containing the system's preferred language.
This leads to localized routing errors or failures on the server.

**Solution**:
We monkey-patch `window.fetch` and `XMLHttpRequest.prototype.send` in our injected script to intercept all outgoing API requests and force the `Accept-Language` header to `"pt-BR,pt;q=0.9"`.

---

## Performance Optimizations

To reduce WebView loading latency and provide a fast reading experience, two critical optimizations are applied:

### 1. Skipping Homepage Pre-load
Normally, a background WebView needs to load the website homepage to acquire Cloudflare clearance and generate session cookies.
This introduces a significant overhead (1.5 to 2.5 seconds) on every chapter request.

**Solution**:
We inspect if the session cookie (`toon_v`) is already cached.
If cached cookies are present, we bypass loading `https://toonlivre.net/` and navigate the WebView directly to the target chapter URL.

### 2. Spoofing Trusted Events
ToonLivre's activation script ignores automated page events to block scrapers, checking `n.isTrusted` before running page hydration.
Without real user interactions, page hydration is deferred until a `3500ms` fallback timeout expires.

**Solution**:
We override `Event.prototype.isTrusted` to return `true` at document start:
`Object.defineProperty(Event.prototype, 'isTrusted', { configurable: true, get: () => true })`

When we dispatch simulated `scroll` and `mousemove` events, the script validates them as trusted user actions and triggers the chapter API fetch immediately, saving up to 3.5 seconds.
