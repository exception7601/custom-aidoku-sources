# ToonLivre Source & Tests

This directory contains the ToonLivre extension source written in Rust and compiled to WebAssembly (WASM).

## Current Architecture (v2.0.0)

As of July 21, 2026, ToonLivre removed encryption from their API. The source now uses a proxy server that:

1. **Direct API access** - No encryption required (default mode)
2. **Automatic encryption fallback** - Detects if encryption returns and switches automatically
3. **Direct image URLs** - Returns CDN URLs for client-side access with proper Referer headers

### Proxy Configuration

The source connects to the proxy server defined in `src/api.rs`:

```rust
fn get_proxy_base() -> String {
    #[cfg(test)]
    {
        // Tests use localhost
        String::from("http://localhost:4000/api")
    }

    #[cfg(not(test))]
    {
        // Production uses remote proxy
        String::from("https://toons.4nd.xyz/api")
    }
}
```

### How It Works

1. **Manga listing/search**: Source calls proxy endpoints
2. **Chapter pages**: Proxy returns array of direct CDN URLs
3. **Image loading**: Source adds `Referer: https://toonlivre.net/` header

## Integration Tests

### Running Tests

Tests require the proxy server running on `http://localhost:4000`:

```bash
# Start proxy server (in toons-total-proxy directory)
cd ../../toons-total-proxy
PORT=4000 bun run dev

# Run integration tests (in source directory)
cd sources/pt_BR.toonlivre
./integration-test.sh
```

### Available Tests

- `live_fetch_releases` - Tests release listing
- `live_search_mangas` - Tests manga search
- `live_fetch_manga_by_slug` - Tests search by slug
- `live_fetch_manga_by_id` - Tests search by ID
- `live_fetch_manga_reader` - Tests reader endpoint
- `live_fetch_chapter` - Tests chapter with image URLs

## API Structure

### Proxy Response Format

All proxy endpoints return:

```json
{
  "success": true,
  "data": { /* actual data */ },
  "timestamp": "2026-07-27T21:00:00.000Z"
}
```

### Chapter Details Response

```json
{
  "success": true,
  "data": {
    "id": "cap-123",
    "mangaId": "obra-456",
    "number": "1",
    "title": "Chapter 1",
    "pages": [
      "https://cdn.toonlivre.net/obras/obra-456/123/page-01.webp",
      "https://cdn.toonlivre.net/obras/obra-456/123/page-02.webp"
    ]
  }
}
```

### Image Request Headers

The source implements `ImageRequestProvider` to add proper headers:

```rust
impl ImageRequestProvider for ToonLivre {
    fn get_image_request(&self, url: String, context: Option<PageContext>) -> Result<Request> {
        let mut request = Request::get(&url)?
            .header("User-Agent", "Mozilla/5.0 ...")
            .header("Accept", "image/avif,image/webp,image/*,*/*;q=0.8")
            .header("accept-language", ACCEPT_LANGUAGE);
        
        let referer = context
            .as_ref()
            .and_then(|ctx| ctx.get("referer"))
            .map(String::as_str)
            .unwrap_or(crate::BASE_URL);
        
        request.set_header("Referer", referer);
        Ok(request)
    }
}
```

## Proxy Server Features

The proxy server (`toons-total-proxy`) provides:

- ✅ Direct API access (no encryption)
- ✅ Automatic encryption fallback (if ToonLivre brings it back)
- ✅ Access logging with IP, response time, status
- ✅ Smart caching (20s TTL)
- ✅ Monitoring endpoints

See `../../toons-total-proxy/README.md` for proxy documentation.

## Migration Notes

### From Token-Server Architecture

Previous versions (before 2.0.0) used a token-server for Rabbit encryption. The new architecture:

**Before (with encryption):**
```
Source → Token Server → Encrypted API → Decrypt → Source
```

**Now (direct access):**
```
Source → Proxy → Direct API → Source
```

**With automatic fallback:**
```
Source → Proxy → [Try Direct] → Success ✓
                ↓ (if fails)
                [Use Token Server] → Success ✓
```

### Deprecated Files

The following files are kept for reference but not actively used:

- `src/token_server.rs` - Token server client (used by proxy fallback)
- `TOKEN_SERVER.md` - Token server documentation
- `TOKEN_SERVER_INTEGRATION.md` - Integration guide

## Building

```bash
# Development build
cargo build --target wasm32-unknown-unknown

# Release build
cargo build --target wasm32-unknown-unknown --release

# Run tests
cargo test

# Integration tests (requires proxy on :4000)
./integration-test.sh
```

## Deployment

The source package (`.aix`) is generated with:

```bash
cargo build --release --target wasm32-unknown-unknown
# Package created at: target/wasm32-unknown-unknown/release/pt_br_toonlivre.wasm
```

## Troubleshooting

### Tests Failing

1. Ensure proxy is running: `curl http://localhost:4000/health`
2. Check proxy logs for errors
3. Verify API is accessible: `curl http://localhost:4000/api/releases?page=1&limit=1`

### Images Not Loading

1. Check browser console for CORS errors
2. Verify Referer header is being sent
3. Test direct CDN access with curl:
   ```bash
   curl -H "Referer: https://toonlivre.net/" "https://cdn.toonlivre.net/..."
   ```

### Proxy Connection Issues

1. Check proxy status: `GET /health`
2. View logs: `GET /api/logs?limit=50`
3. Check encryption mode: `GET /api/encryption/status`

## Documentation

- Main README: `../../README.md`
- Proxy documentation: `../../toons-total-proxy/PROJETO-FINALIZADO.md`
- Quick commands: `../../toons-total-proxy/COMANDOS-RAPIDOS.sh`
