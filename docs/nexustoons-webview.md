# NexusToons WebView Strategy

This document tracks the current `pt_BR.nexustoons` strategy for list APIs, WebView capture, and Playwright validation.

## Current file layout

The `NexusToons` source is now split across these files:

- `sources/pt_BR.nexustoons/src/source/mod.rs`
- `sources/pt_BR.nexustoons/src/source/manga.rs`
- `sources/pt_BR.nexustoons/src/source/webview.rs`
- `sources/pt_BR.nexustoons/src/source/webview_support.rs`
- `sources/pt_BR.nexustoons/src/webview/instrumentation.js`
- `sources/pt_BR.nexustoons/playwright/nexustoons-instrumentation.js`
- `sources/pt_BR.nexustoons/playwright/instrumentation.smoke.spec.js`
- `sources/pt_BR.nexustoons/playwright/instrumentation.unit.spec.js`

`mod.rs` owns the Aidoku `Source` and `Home` implementations.
`manga.rs` owns list mapping and chapter normalization.
`webview.rs` owns the manga-detail and chapter-page orchestration.
`webview_support.rs` owns cache parsing, config serialization, and layout/debug helpers.
`instrumentation.js` is the shared document-start script used both by Aidoku WebView and by Playwright.

## What the site exposes directly

The public list API is still straightforward.
The verified endpoints are:

- `GET /api/mangas?page={page}&limit={limit}`
- `GET /api/mangas?search={query}&page={page}&limit={limit}`

Those responses are mapped directly into Aidoku list entries.

The detail and reader flows are not equally friendly.
`/api/manga/{id}` returns an encrypted wrapper such as `{ "d": "...", "k": 2, "v": 2 }`.
Direct chapter endpoints such as `/api/chapter/{id}` return unauthorized responses when requested outside the page flow.
The HTML pages are mostly SPA shells, so plain `curl` on `/manga/...` and `/ler/...` does not expose the decoded reader state or page URLs server-side.

Because of that, this source still uses a WebView for manga details and chapter pages.
This pass intentionally avoids reimplementing the site decrypt logic in Rust.
Instead, it captures the decoded runtime data inside the browser environment and keeps the JS side more testable.

## Shared instrumentation strategy

`src/webview/instrumentation.js` exposes:

- `globalThis.__nexustoonsAidokuBoot(config)`
- `globalThis.__nexustoonsAidokuApplyLayoutPatch(config)`
- `globalThis.__nexustoonsAidokuOpenChapterList()`
- `globalThis.__nexustoonsAidokuCaptureMangaNow()`
- `globalThis.__nexustoonsAidokuCollectChapterPagesNow()`

The Rust side injects the file through `WebViewUserScript`.
The Playwright side injects the exact same file through `context.addInitScript(...)`.

The script is organized as a generic core plus a thin `NexusToons` adapter config.
The generic core owns:

- layout and visibility patching
- `Accept-Language` normalization
- JSON, `fetch`, and `XMLHttpRequest` hooks
- recursive payload extraction helpers
- chapter-image collection helpers
- cache persistence helpers
- testable pure internals

The adapter config provides site-specific hints such as:

- `siteHostHints`
- `mangaPageUrlHints`
- `chapterPageUrlHints`
- `mangaPayloadUrlHints`
- `chapterButtonTextHints`
- `chapterLinkUrlHints`
- `reactRootSelectors`
- `chapterImageUrlHints`

## Cache contract

The shared script persists two caches.

For manga details:

- global key `__nexustoonsAidokuMangaCache`
- storage key format `nexustoons_manga_cache_v1:{slug}`

For chapter page URLs:

- global key `__nexustoonsAidokuChapterPagesCache`
- storage key format `nexustoons_chapter_pages_cache_v1:{slug}:{chapterId}`

Rust reads from the global first and falls back to `sessionStorage`.
That makes the contract consistent with Playwright and removes the old inline `eval` scripts that rebuilt extraction logic on every poll.

## Manga detail capture flow

For manga pages, the shared script now does the following:

- patches the page to look like a visible iPhone-sized WebKit surface
- hooks `JSON.parse`, `fetch`, and `XMLHttpRequest`
- clicks the `Capítulos` button using configurable text hints
- recursively searches decoded objects for a likely manga payload shape
- falls back to traversing React roots from configurable DOM seeds
- normalizes the result into `{ title, coverUrl, description, status, chapters }`
- persists the best payload by chapter count

The important point is that the capture is now shape-based rather than tied to one exact object path.
If the site moves the decoded object deeper inside a wrapper, the generic extractor still has a good chance of finding it.

## Chapter page capture flow

For chapter pages, the shared script now does the following:

- patches the same mobile visibility/fingerprint surface
- scans image elements for URLs matching `chapterImageUrlHints`
- scrolls the page in scheduled steps to trigger lazy rendering
- keeps collecting URLs through a mutation observer
- persists the best page list into the shared chapter cache

This replaces the old Rust-side pattern of repeatedly running one large inline collector script.
The Rust side now just nudges the shared helpers and waits for the cache.
That keeps the logic closer to the browser and makes it reusable in Playwright.

## Rust-side polling and debug behavior

Rust keeps a conservative whole-second polling loop because `sleep` only supports integer seconds.
The current constants remain:

- `WEBVIEW_LOAD_ATTEMPTS = 4`
- `WEBVIEW_LOAD_DELAY_SECONDS = 1`

That gives one immediate check plus up to three delayed retries.
The shared JS now performs earlier scheduled capture work, so the wait loop mostly polls the cache instead of rebuilding the extraction logic.

Heavy debug logging is opt-in in release builds.
`enable_debug_logs=1` or `debug_assertions` enables the full debug snapshot path.
Without that flag, the functional hooks still run, but the expensive logging path stays off.

## Playwright coverage

A local Playwright setup now exists under `sources/pt_BR.nexustoons`.
It validates the shared `instrumentation.js` in real browsers and keeps the browser-side behavior close to the Aidoku WebView path.

The helper creates an iPhone-like browser context and injects the same boot config shape that Rust uses.
It also uses conditional waits for cache readiness instead of fixed sleeps.

The current smoke suite validates:

- manga-detail capture from the real `/manga/...` flow
- chapter page URL capture from the real `/ler/...` flow
- chapter page capture with debug disabled

The current unit suite validates:

- nested manga payload extraction by shape
- normalization of aliases such as `coverImage` and chapter metadata
- image URL filtering and deduplication
- site-scoped endpoint matching for manga payload requests

## Useful commands

For Rust validation:

- `env -C sources/pt_BR.nexustoons cargo fmt`
- `env -C sources/pt_BR.nexustoons cargo test helper_parses_webview_manga_cache -- --nocapture`
- `env -C sources/pt_BR.nexustoons cargo test helper_parses_webview_chapter_pages_cache -- --nocapture`
- `env -C sources/pt_BR.nexustoons cargo clippy`

For Playwright validation:

- `env -C sources/pt_BR.nexustoons npm install`
- `env -C sources/pt_BR.nexustoons npm run playwright:install`
- `env -C sources/pt_BR.nexustoons npm run test:unit`
- `env -C sources/pt_BR.nexustoons npm run test:playwright:webkit`
- `env -C sources/pt_BR.nexustoons npm run test:playwright:chromium`
