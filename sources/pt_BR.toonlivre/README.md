# ToonLivre Source & Tests

This directory contains the ToonLivre Aidoku source written in Rust and compiled to WebAssembly.

## Overview

The source talks directly to `https://toonlivre.net/api`.
It does not use the local proxy server anymore.

The direct flow is:

- Call list and manga endpoints without `seed` when the public API is available.
- Fall back to `/api/seed` plus `toon_v` and `x-toon-signature` if a metadata endpoint returns an error.
- Load chapter pages in a background `WebView`.
- Read the decrypted chapter cache from `sessionStorage`.

## Direct API endpoints

The source uses these endpoints for listing and manga metadata:

- `GET /api/mangas/releases`
- `GET /api/mangas/search`
- `GET /api/manga-by-slug/:slug`
- `GET /api/mangas/:id`
- `GET /api/mangas/:id/reader`

## Tests

Run the full source test suite with:

```bash
cd /Users/anderson/Developer/custom-aidoku-sources/sources/pt_BR.toonlivre
cargo test
```

The live tests require network access to `toonlivre.net`.

## Build

```bash
cargo build --target wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

## Image loading

The source sets a `Referer` header for chapter images so the CDN accepts the request.

## Troubleshooting

If live tests fail, check that `toonlivre.net` is reachable from your machine.
A `404` on `/api/seed` is expected while the public metadata endpoints remain available.
If chapter loading fails, inspect the WebView cache key and the page request headers.
