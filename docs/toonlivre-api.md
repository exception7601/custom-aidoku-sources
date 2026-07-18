# ToonLivre API notes

The `ToonLivre` source is implemented against the site API.
The HTML pages mainly bootstrap the JavaScript application.
The useful source data comes from `https://toonlivre.net/api`.

## Main endpoints

These endpoints were enough to implement the current source.

- `GET /api/mangas/releases?page=<page>&limit=<limit>`
- `GET /api/mangas/search?q=<query>&page=<page>&limit=<limit>&sortBy=updated&sortOrder=desc`
- `GET /api/manga-by-slug/<slug>`
- `GET /api/mangas/<mangaId>`
- `GET /api/mangas/<mangaId>/reader`
- `GET /api/mangas/<mangaId>/chapters/<chapterId>`

## Endpoint roles

### Releases

`/api/mangas/releases` is the best entry point for the home feed and for paginated browsing.
It returns:

- manga cards
- recent chapter summaries
- pagination metadata

The current source uses that endpoint for:

- home content
- generic listing pagination when there is no search query

### Search

`/api/mangas/search` is the best entry point for title search.
The current source sends:

- `q`
- `page`
- `limit`
- `sortBy=updated`
- `sortOrder=desc`

### Manga lookup by slug

`/api/manga-by-slug/<slug>` is the easiest way to start from a public manga URL.
It returns the internal manga id and a full chapter list.
That makes it a good bridge from browser-style URLs to API-style ids.

### Manga lookup by id

`/api/mangas/<mangaId>` is useful once the internal id is known.
It returns normalized metadata, including the canonical slug.
The source uses it when the current manga key is already the internal id.

### Reader metadata

`/api/mangas/<mangaId>/reader` returns the full chapter list in guest mode.
This endpoint is more useful than `/chapters-paginated` for Aidoku.

In guest testing, `/api/mangas/<mangaId>/chapters-paginated` returned `403` with `Sessão inválida ou expirada`.
Because of that, the source does not depend on the paginated chapter endpoint.

### Chapter payload

`/api/mangas/<mangaId>/chapters/<chapterId>` returns the page list for a single chapter.
This response is encrypted.
See `docs/toonlivre-security.md` for the decryption flow.

## Required headers

The current source mirrors the frontend request shape.
These headers are the important ones.

### Safe defaults for JSON endpoints

- `Accept: application/json, text/plain, */*`
- `Accept-Language: en-US,en;q=0.9,pt;q=0.8`
- `Origin: https://toonlivre.net`
- `Referer: https://toonlivre.net`

For most public JSON endpoints, the frontend also sends:

- `toonlivre-pass: decoy99xz`
- `x-toon-verify: <token>`
- `Cookie: toon_v=<same token>`

In practice, some public endpoints still answer without the custom headers.
The source keeps the request shape close to the frontend anyway.

### Required additions for chapter endpoints

For chapter endpoints, the important extras are:

- `toonlivre-pass: auth2028xy`
- `x-toon-verify: <token>`
- `Cookie: toon_v=<same token>`
- `Accept-Language: en-US,en;q=0.9,pt;q=0.8`

The chapter endpoint rejected requests that did not provide the expected custom header pattern.
Using the same token value in `x-toon-verify` and `toon_v` was sufficient in guest mode.

## Example requests

### Releases

```sh
curl 'https://toonlivre.net/api/mangas/releases?page=1&limit=48' \
  -H 'accept: application/json, text/plain, */*' \
  -H 'accept-language: en-US,en;q=0.9,pt;q=0.8'
```

### Search

```sh
curl 'https://toonlivre.net/api/mangas/search?q=duque&page=1&limit=24&sortBy=updated&sortOrder=desc' \
  -H 'accept: application/json, text/plain, */*' \
  -H 'accept-language: en-US,en;q=0.9,pt;q=0.8'
```

### Chapter payload

```sh
TOKEN='aidoku-toonlivre'

curl 'https://toonlivre.net/api/mangas/obra-dbbabf0f/chapters/cap-dd9e898d-522_5' \
  -H 'accept: application/json, text/plain, */*' \
  -H 'accept-language: en-US,en;q=0.9,pt;q=0.8' \
  -H 'origin: https://toonlivre.net' \
  -H 'referer: https://toonlivre.net' \
  -H 'toonlivre-pass: auth2028xy' \
  -H "x-toon-verify: ${TOKEN}" \
  -H "cookie: toon_v=${TOKEN}"
```

## Image loading

The decrypted chapter payload contains direct CDN image URLs under `https://cdn.toonlivre.net/...`.
Those URLs can be used directly in the Aidoku page list.

The source still sends a browser-like image request with:

- `Accept: image/avif,image/webp,image/*,*/*;q=0.8`
- `Accept-Language: en-US,en;q=0.9,pt;q=0.8`
- `Referer: <chapter URL>`

That keeps image loading closer to the browser path.
