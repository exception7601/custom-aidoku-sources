# NexusToons WebView Strategy

This document describes how `pt_BR.nexustoons` loads manga details and chapters from `nexustoons.com`.

## API flow

Home mirrors the simpler `pt_BR.montetaiscanlator` pattern and uses a single manga list component backed by the public JSON API.
Search uses the same API with pagination.
The verified endpoints are:

- `GET /api/mangas?page={page}&limit={limit}`
- `GET /api/mangas?search={query}&page={page}&limit={limit}`

The API returns a paginated payload with `data`, `page`, `pages`, and `total`.
The source maps that payload directly into Aidoku manga entries.

Home uses `GET /api/mangas?page={page}&limit={limit}` and renders one manga grid/list component.
Search uses `GET /api/mangas?search={query}&page={page}&limit={limit}` and keeps infinite scrolling by advancing `page` while preserving `limit`.

## WebView flow for manga details

Manga detail pages are loaded in a background `WebView`.
The page already contains the decoded manga object in browser runtime state.
The source captures that object instead of reproducing any decrypt logic in Rust.

The WebView script does three things.

- It patches visibility and viewport checks so the page hydrates in the background.
- It intercepts `JSON.parse`, `fetch`, and `XMLHttpRequest` to capture manga JSON.
- It opens the chapter list and retries capture a few times until the full object is available.

## Captured payload shape

The captured JSON is intentionally sanitized before being sent back to Rust.
Only the fields needed by the source are kept.

- `title`
- `coverUrl`
- `description`
- `status`
- `chapters`

Each chapter keeps only:

- `id`
- `number`
- `title`
- `date_uploaded`

This avoids duplicate field errors and keeps the parser tolerant of missing or nullable site data.

## Rust-side handling

Rust maps the captured manga object into Aidoku structs.
Chapter titles fall back to `Capítulo {number}` when empty.
Chapter timestamps are not trusted from the page capture and are filled with the current date.

## Notes

The implementation is designed to stay generic and stable.
It avoids site-specific decrypt code and relies on browser-captured JSON plus the documented API endpoints.
