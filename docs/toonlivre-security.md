# ToonLivre security notes

`ToonLivre` uses a mix of request gating and payload obfuscation.
It is not a plain HTML site.
The frontend is a JavaScript app that talks to JSON endpoints under `https://toonlivre.net/api`.

For the Aidoku source, the important part is that chapter data is not returned as plain JSON.
The chapter endpoint requires extra headers and returns an encrypted payload.

## High-level model

There are two practical classes of requests.

- Public JSON endpoints.
- Protected chapter endpoints.

Public JSON endpoints include releases, search, manga lookup, and reader metadata.
These usually return normal JSON.

Protected chapter endpoints return a JSON object with an encrypted string.
The source must decrypt that string to recover the page URLs.

## Request gating

The frontend adds a custom header named `toonlivre-pass` to every API request.
The value depends on the endpoint class.

- Regular API endpoints use `decoy99xz`.
- Chapter endpoints use `auth2028xy`.

The frontend also sends a visitor token twice.

- `x-toon-verify: <token>`
- `Cookie: toon_v=<token>`

In live testing, the chapter endpoint accepted arbitrary token values as long as the header and cookie matched.
That means this mechanism behaves more like a consistency check than a strong per-user secret.

The source uses a stable token value and sends it in both places.
That is enough for the guest chapter API path.

## Encrypted chapter payload

Chapter requests use this endpoint shape.

- `GET /api/mangas/<mangaId>/chapters/<chapterId>`

A successful response includes an `x-toon-datakey` response header.
The body is a JSON object whose property name matches that header.
The property value is a base64 string that starts with `Salted__` after decoding.

That format matches the usual CryptoJS/OpenSSL salted container layout.
The decrypted plaintext is JSON.
For chapter responses, that plaintext contains the `pages` array used by the source.

## Daily passphrase derivation

The encrypted payload is not decrypted with a fixed literal password.
The frontend derives a daily passphrase.
The source reproduces the same logic.

The derivation steps are:

- Get the current UTC date as `YYYY-MM-DD`.
- Concatenate `date + "toonlivre.tv::v8" + "t17_4v19_b2"`.
- Compute the MD5 digest of that string.
- Take the first 8 hex characters of the digest.
- Prefix them with `Dealer-Critter-Catnip4`.

The final passphrase looks like this shape.

- `Dealer-Critter-Catnip4xxxxxxxx`

Where `xxxxxxxx` is the first 8 hex characters of the daily MD5 digest.

## Cipher details

After base64 decoding and salt extraction, the source derives a Rabbit key and IV with the OpenSSL-style `EVP_BytesToKey` process.
The implementation uses repeated MD5 blocks until it has enough bytes.

The final split is:

- 16 bytes for the Rabbit key.
- 8 bytes for the Rabbit IV.

The ciphertext is then decrypted with the Rabbit stream cipher.
The plaintext is UTF-8 JSON.

## Operational consequences

The source should be treated as API-first, not HTML-first.
Scraping rendered HTML would be less stable than following the same contract as the frontend.

For chapter lists, `GET /api/mangas/<mangaId>/reader` is more useful than `GET /api/mangas/<mangaId>/chapters-paginated`.
In guest testing, the paginated chapter endpoint returned `403` with `Sessão inválida ou expirada`, while the reader endpoint returned the full chapter list.

CDN image URLs are public once the decrypted chapter payload is available.
The source can load them directly after building the page list.
