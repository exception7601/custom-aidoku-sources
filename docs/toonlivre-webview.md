# ToonLivre WebView and API Decryption Strategy

This document tracks the current ToonLivre chapter loading strategy, the shared WebView instrumentation, and the browser-side smoke tests.

## Current file layout

The WebView flow is split across these files:

- `sources/pt_BR.toonlivre/src/source/webview.rs`
- `sources/pt_BR.toonlivre/src/source/webview_support.rs`
- `sources/pt_BR.toonlivre/src/webview/instrumentation.js`
- `sources/pt_BR.toonlivre/playwright/toonlivre-instrumentation.js`
- `sources/pt_BR.toonlivre/playwright/instrumentation.smoke.spec.js`

`webview.rs` is the Rust orchestrator.
`webview_support.rs` owns the debug snapshot structs, cookie helpers, config serialization, and JS bootstrapping helpers.
`instrumentation.js` is the shared document-start script used both by Aidoku WebView and by Playwright.

## Runtime strategy

ToonLivre protects chapter access behind client-side runtime logic.
The page bootstraps a reader flow, spawns runtime workers under `/api/reader/runtime`, verifies proof data under `/api/reader/proof/verify`, and only then emits the chapter payload with `pages`.

Instead of reimplementing that moving logic in Rust, the source loads the chapter inside a background `WebView` and instruments the browser environment early enough for the site code to run normally.

The chapter payload is persisted under the storage key format `toonlivre_chapter_cache_v1:{mangaId}:{chapterId}`.
The payload is saved both in `sessionStorage` and in `window.__toonlivreAidokuChapterCache`.
Rust reads that cache back and converts it into `ApiChapterDetails`.

## What the shared instrumentation does

`src/webview/instrumentation.js` exposes `globalThis.__toonlivreAidokuBoot(config)` and `globalThis.__toonlivreAidokuApplyLayoutPatch(config)`.
The Rust side injects the file with a serialized config object through `WebViewUserScript`.
The Playwright side injects the exact same file with `context.addInitScript(...)`.

The boot script currently does all of the following:

- patches visibility so the page behaves as if it is foregrounded
- patches a coherent iPhone-like fingerprint at `390x844 @3x`
- sets `navigator.userAgent`, `platform`, `vendor`, and `maxTouchPoints`
- forces `Accept-Language` to `pt-BR,pt;q=0.9`
- ensures `toon_v` is present in `document.cookie`
- hooks `fetch` and `XMLHttpRequest`
- wraps `Worker` and instruments the runtime worker flow
- captures runtime and proof debug events in `window.__toonlivreAidokuDebug`
- captures chapter payloads from worker messages, `fetch`, and `XMLHttpRequest`
- persists the normalized chapter payload into the shared cache keys

## Important implementation details

### Visibility and fingerprint patching

A default background `WKWebView` reports hidden visibility and unusable dimensions.
That causes hydration and reader startup to stall.
The shared script patches the page before site scripts run so the app sees a visible mobile browser instead of a hidden zero-sized surface.

### Cookie handling

The source still prepares a manual `Cookie` header so `toon_v` is available on the initial chapter navigation.
The shared script also sets `toon_v` in `document.cookie` so site-side JavaScript can read it immediately.

### Runtime worker proxying

The critical reader logic moved into runtime workers.
Because of that, main-thread hooks are not enough.
The shared script wraps `Worker`, fetches the runtime source, builds a blob worker with a debug shim, and forwards worker-side fetch, XHR, import, and message events back into the page debug store.

### Chapter cache persistence

The source no longer depends on a single code path to populate chapter pages.
The instrumentation watches:

- worker messages
- chapter `fetch` responses
- chapter `XMLHttpRequest` responses

Any valid chapter payload is normalized and persisted, which makes the Rust side much less fragile.

## Homepage preload status

Older iterations loaded `https://toonlivre.net/` before the chapter page.
That was useful while the runtime and cookie behavior were still unclear.

After adding the shared Playwright WebKit smoke tests, the direct chapter navigation path was verified successfully.
The Rust flow now opens the chapter URL directly and no longer preloads the homepage.

## Playwright coverage

A lightweight Playwright setup now exists directly under `sources/pt_BR.toonlivre`.
It is meant to validate the shared `instrumentation.js`, not to replace final validation in the Aidoku app.

The Playwright helper:

- creates a mobile WebKit or Chromium context
- applies the same user agent and viewport profile as the Aidoku WebView flow
- injects `src/webview/instrumentation.js`
- boots it with the same config shape used by Rust
- opens a real ToonLivre chapter URL
- waits until the chapter cache is populated
- inspects the debug store and worker events

The current smoke test checks two scenarios:

- with a homepage visit before the chapter
- without a homepage visit before the chapter

The WebKit run passed in both scenarios, which is why the Rust path was simplified to direct chapter navigation.

## Resource blocking in Playwright

The Playwright helper blocks non-essential resource types to reduce smoke-test time.
The current safe block list includes:

- `image`
- `media`
- `font`
- `stylesheet`
- `texttrack`
- common analytics and ads hosts

A more aggressive allowlist-based strategy was benchmarked.
It did not improve the WebKit run and was reverted.
The safe blocker is the current default.

## What the Playwright smoke test proves

The browser-side smoke test proves that:

- the shared instrumentation file loads correctly
- `__toonlivreAidokuBoot(config)` executes at document start
- the runtime worker flow still reaches chapter data
- the chapter cache is populated with non-empty `pages`
- the debug store records runtime, proof, and worker activity
- the direct chapter navigation path works in WebKit without a homepage preload

It does not prove that Aidoku itself is fully correct on-device.
Final validation should still use `aidoku logcat` against the packaged source.

## Useful commands

For Rust validation:

- `env -C sources/pt_BR.toonlivre cargo fmt`
- `env -C sources/pt_BR.toonlivre cargo test helper_parses_webview -- --nocapture`
- `env -C sources/pt_BR.toonlivre cargo clippy`
- `env -C sources/pt_BR.toonlivre aidoku package`

For Playwright validation:

- `env -C sources/pt_BR.toonlivre npm install`
- `env -C sources/pt_BR.toonlivre npm run playwright:install`
- `env -C sources/pt_BR.toonlivre npm run test:playwright:webkit`
- `env -C sources/pt_BR.toonlivre npm run test:playwright:chromium`
