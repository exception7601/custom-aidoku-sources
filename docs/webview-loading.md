# WebView Loading Strategy for ToonLivre

This document describes how the WebView chapter loader works, the issues that were resolved, and potential optimization steps to improve performance.

## How WebView Loading Works

The ToonLivre source utilizes a background `WebView` to retrieve chapter pages.
This approach is necessary because the website encrypts its chapter pages payload using the Rabbit cipher and dynamically evaluates the decryption key on the client side.
The site's JavaScript automatically decrypts the pages and stores them in `sessionStorage` under the key format:
`toonlivre_chapter_cache_v1:{mangaId}:{chapterId}`

The Rust source creates a `WebView`, navigates to the chapter page, and periodically queries `sessionStorage` via `webview.eval` until the cache is populated.

## Resolved Issues

Three major issues prevented the WebView from loading pages correctly on target devices:

### 1. Visibility and Layout Restraints
When Aidoku instantiates a background `WebView`, the page visibility state is default to `"hidden"` and the layout size is `0x0`.
Under these constraints, the Next.js and React hydration cycles either crash or suspend rendering of the reader component.
This prevented the script from performing the API fetch and saving the data to `sessionStorage`.

**Solution**:
We injected a `WebViewUserScript` at document start to mock the DOM properties, spoofing visibility to `"visible"`, `hidden` to `false`, and dimensions to `1280x1920`.
We also dispatched simulated `scroll`, `mousemove`, and `focus` events to trigger the website's activation handlers.

### 2. Session Cookie Requirements
ToonLivre's API endpoints require a session identification cookie (`toon_v` or `toon_i`).
If this cookie is missing on the initial document request, the Next.js server fails to load the initial props and returns a "Mangá não encontrado" (Manga not found) error page.

**Solution**:
We ensured that the manual `toon_v` cookie header is passed on both the base homepage request and the final chapter page request.

### 3. Locale and Accept-Language Mismatch
When the target device has a system locale other than Portuguese (e.g., `en-GB`), the WebView's subresource API calls default to the system locale.
This caused the server to return localized routing errors or fail the request.

**Solution**:
We monkey-patched `window.fetch` and `XMLHttpRequest.prototype.send` inside the WebView to intercept all outgoing API requests and force the `Accept-Language` header to `"pt-BR,pt;q=0.9"`.

---

## How to Improve Loading Performance

Currently, the WebView loading process takes approximately 2 to 4 seconds, as it needs to load the site skeleton, run scripts, fetch the API, and populate the cache.
Here are the recommended strategies to optimize performance and reduce load times:

### 1. Remove the Homepage Pre-load Step
Currently, we make a blocking request to `BASE_URL` before loading the chapter URL to prime Cloudflare clearance and cookies.

- **Pros**: Ensures Cloudflare clearance is obtained and cookies are set.
- **Cons**: Adds a round-trip delay of 1.5 to 2.5 seconds.
- **Action**: We can attempt to navigate directly to the chapter URL with a pre-generated session cookie (`toon_v`), skipping the homepage load entirely if the User-Agent is stable.

### 2. Native Rust Decryption (Proxy / Direct API)
Instead of relying on the WebView to load, render, and decrypt the chapter pages, we can fetch the encrypted chapter API endpoint directly in Rust or through the proxy server, and decrypt the payload natively.

- **Pros**: Reduces load time from seconds to milliseconds, bypassing the WebView entirely.
- **Cons**: Requires replicating the dynamically changing route tokens and the UTC hour-based decryption key.
- **Action**: Implement Rabbit cipher decryption in Rust or let the `toons-proxy` server handle the decryption by querying `/api/chapter-token` and executing the decryption routine using `CryptoJS` on the proxy side.

### 3. Intercept and Extract Response Directly in WebView
If the WebView is still needed for Cloudflare clearance, we can listen to the fetch response directly inside the WebView instead of waiting for React to mount and populate `sessionStorage`.

- **Pros**: Bypasses the React hydration and rendering cycle.
- **Cons**: Requires complex injection of service worker overrides or monkey-patching `Response.json`.
