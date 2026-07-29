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

### 1. Viewport Visibilty and Layout Constraints
Background WebViews on iOS (WKWebView) load with a visibility state of `"hidden"` and dimensions of `0x0`.
ToonLivre's React application suspends rendering or fails hydration under these conditions.

**Solution**:
We inject a `WebViewUserScript` at document start to mock the DOM properties before any page script executes.

This script patches:
- `document.visibilityState` to `"visible"`
- `document.hidden` to `false`
- `window.innerWidth` and `window.innerHeight` to `1280x1920`
- `window.outerWidth` and `window.outerHeight` to `1280x1920`

### 2. Trusted Events Protection
ToonLivre implements a protection script that ignores programmatic window events.
It checks the `isTrusted` property on events, which is `false` for events dispatched via `window.dispatchEvent`.

**Solution**:
We redefine `Event.prototype.isTrusted` inside the user script to always return `true`.
This allows simulated `scroll`, `mousemove`, and `focus` events to be recognized as user actions, immediately triggering hydration and API fetch calls.

### 3. Session and Cookie Requirements
ToonLivre requires a session cookie (`toon_v`) on the initial document request.
If the cookie is absent, Next.js server-side rendering renders a "Manga not found" error page instead of the chapter reader.

**Solution**:
We pass the manual `Cookie` header containing a generated session ID on all WebView requests.
Subsequent chapter requests skip the homepage load and navigate directly using the cached session cookie.

### 4. Language Routing Obstacles
If the device locale is set to a non-Portuguese language, the WebView makes subresource API requests containing the system's preferred language.
This leads to localized routing errors or failures on the server.

**Solution**:
We monkey-patch `window.fetch` and `XMLHttpRequest.prototype.send` in our injected script to intercept all outgoing API requests and force the `Accept-Language` header to `"pt-BR,pt;q=0.9"`.

---

## Direct API Decryption (Proxy Implementation)

For API requests outside of the WebView (such as manga list updates), the `toons-proxy` server handles encryption/decryption natively.

The proxy executes the following sequence:

- Generates an hourly passphrase based on the MD5 hash of the current UTC timestamp:
  - `UTC = Date.UTC(year, month, day, hour)`
  - `Passphrase = MD5(UTC).substring(0, 8)`
- Fetches the dynamic route token by querying the `/api/chapter-token` endpoint with signature headers.
- Makes the request to the encrypted chapter endpoint `/api/mangas/{mangaId}/chapters/{chapterId}` passing the route token.
- Decrypts the response natively using standard `CryptoJS.Rabbit.decrypt` with the hourly passphrase.
