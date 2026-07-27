# Toons Total Proxy v2.0.0

JSON proxy server for the Toons Total (ToonLivre) API with simplified REST endpoints.

## 🎯 Features

- **Direct API access** (no encryption required - as of July 21, 2026)
- **Automatic encryption fallback** - Detects when encryption is needed and switches automatically
- **Access logging system** - Tracks IP, response time, status, errors
- **Smart caching** with 20s TTL for API requests
- **Chapter endpoint** returns direct CDN image URLs
- **Complete TypeScript types**
- **Comprehensive tests** (19/19 passing)
- **Docker ready** with multi-stage build
- **Linter and Formatter** with Biome

## Stack

- Runtime: Bun
- Framework: Elysia
- Language: TypeScript
- Linter/Formatter: Biome
- Container: Docker

## Quick Start

### Development

```bash
bun install
bun run dev
bun run test           # Run all tests (19 tests)
bun run test:live      # Run live integration tests
bun run test:fallback  # Run encryption fallback tests
bash scripts/demo-complete.sh  # Demo all features
```

### Production

```bash
PORT=4000 bun run src/index.ts

# With token-server for encryption fallback
TOKEN_SERVER_HOST=http://localhost:3001 PORT=4000 bun run src/index.ts
```

### Docker

```bash
docker-compose up -d
docker-compose logs -f proxy
docker-compose down
```

## Endpoints

### Core API

#### Health Check
```
GET /health
```
Returns server status and encryption mode.

#### Releases
```
GET /api/releases?page=1&limit=48
```

#### Search
```
GET /api/search?q=term&page=1&limit=24
```

#### Manga Details
```
GET /api/manga/:id
```

#### Manga by Slug
```
GET /api/manga-by-slug/:slug
```

#### Manga Reader
```
GET /api/manga/:id/reader
```

#### Chapter Details with Images
```
GET /api/manga/:mangaId/chapters/:chapterId
```

Returns chapter with array of direct CDN image URLs.

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "cap-01",
    "number": "1",
    "pages": [
      "https://cdn.toonlivre.net/obras/obra-123/01/page-01.webp",
      "https://cdn.toonlivre.net/obras/obra-123/01/page-02.webp"
    ]
  }
}
```

**Client usage:** Access image URLs directly with header `Referer: https://toonlivre.net/`

### Monitoring & Control

#### Encryption Status
```
GET /api/encryption/status
```

Returns current mode (direct/encrypted).

#### Toggle Encryption Mode
```
POST /api/encryption/toggle
Body: {"enabled": true}
```

#### Access Logs
```
GET /api/logs?limit=100
```

Returns access logs with IP, method, path, status, response time.

#### Log Statistics
```
GET /api/logs/stats
```

Returns aggregated statistics.

#### Clear Cache
```
POST /api/cache/clear
```

#### Clear Logs
```
DELETE /api/logs
```

## Environment Variables

```bash
PORT=4000                                    # Server port
NODE_ENV=production                          # Environment
TOKEN_SERVER_HOST=http://localhost:3001      # Token-server for encryption fallback
```

## Response Format

Success:
```json
{
  "success": true,
  "data": {},
  "timestamp": "2026-07-26T23:46:10.944Z"
}
```

Error:
```json
{
  "success": false,
  "error": "error message",
  "timestamp": "2026-07-26T23:46:10.944Z"
}
```

## Project Structure

```
toons-total-proxy/
├── src/
│   ├── index.ts            # Main server with logging
│   ├── toonlivre-api.ts    # API client with encryption fallback
│   ├── logger.ts           # Access logging system
│   ├── token-manager.ts    # Token management (used by fallback)
│   └── token-server.ts     # Token server client (used by fallback)
├── tests/
│   ├── api.test.ts         # Type and config tests
│   ├── live.test.ts        # Live integration tests
│   ├── fallback.test.ts    # Encryption fallback tests (12 tests)
│   └── e2e.test.ts         # End-to-end tests
├── scripts/
│   ├── demo-complete.sh    # Complete demo
│   └── test-endpoints.sh   # Quick endpoint tests
├── PROJETO-FINALIZADO.md   # Complete project documentation (PT-BR)
├── COMPLETE-SUMMARY.md     # Technical summary
├── COMANDOS-RAPIDOS.sh     # Quick reference commands
├── Dockerfile
├── docker-compose.yml
├── package.json
├── tsconfig.json
└── biome.json
```

## How It Works

### Automatic Encryption Fallback

The proxy automatically detects when encryption is needed:

1. **Tries direct access first** (current mode - no encryption)
2. **Detects encryption requirements** - If receives 401/403 or token/signature errors
3. **Activates encryption mode** - Connects to token-server automatically
4. **Uses Rabbit encryption** - Applies tokens and decrypts responses
5. **Checks periodically** - Verifies API changes every 5 minutes

### Access Logging

All requests are logged with:
- Client IP address
- HTTP method and path
- Response status and time
- User-Agent
- Error messages (if any)

View logs in real-time:
```bash
curl -s "http://localhost:4001/api/logs?limit=10" | jq
```

### No Image Proxy

The `/api/manga/:id/chapters/:chapterId` endpoint returns direct CDN URLs. Clients access images directly with proper headers:

```javascript
// Client-side code
for (const imageUrl of chapter.pages) {
  fetch(imageUrl, {
    headers: { 'Referer': 'https://toonlivre.net/' }
  })
}
```

## Monitoring

### View Logs
```bash
# Last 50 logs
curl -s "http://localhost:4001/api/logs?limit=50" | jq

# Statistics
curl -s "http://localhost:4001/api/logs/stats" | jq
```

### Check Encryption Status
```bash
# In health endpoint
curl -s "http://localhost:4001/health" | jq '.encryption'

# Dedicated endpoint
curl -s "http://localhost:4001/api/encryption/status" | jq
```

### Manual Control
```bash
# Force encryption mode
curl -X POST "http://localhost:4001/api/encryption/toggle" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'

# Clear cache
curl -X POST "http://localhost:4001/api/cache/clear"

# Clear logs
curl -X DELETE "http://localhost:4001/api/logs"
```

## Deploying

### Docker Registry

```bash
docker tag toons-total-proxy:1.1.0 registry/toons-total-proxy:1.1.0
docker push registry/toons-total-proxy:1.1.0
```

### Docker Compose

```bash
docker-compose up -d
docker-compose logs -f proxy
docker-compose down
```

## Troubleshooting

### Check Encryption Status

```bash
curl -s "http://localhost:4001/health" | jq '.encryption'
```

If `enabled: true`, the API switched to encrypted mode automatically.

### View Recent Errors

```bash
curl -s "http://localhost:4001/api/logs?limit=50" | jq '.data[] | select(.status >= 400)'
```

### Port Already in Use

```bash
PORT=3001 bun run src/index.ts
```

### Type Errors

```bash
bun run type-check
```

### Connection Error

Check ToonLivre API directly:
```bash
curl https://toonlivre.net/api/mangas/releases?page=1&limit=3
```

### Token-Server Not Available

If encryption fallback fails:
1. Check TOKEN_SERVER_HOST environment variable
2. Ensure token-server is running
3. Check logs: `curl -s "http://localhost:4001/api/logs" | jq`

## Testing

### Run All Tests (19 tests)
```bash
bun test
```

### Test Categories
```bash
bun test tests/fallback.test.ts   # 12 encryption fallback tests
bun test tests/live.test.ts       # 7 integration tests
```

### Demo All Features
```bash
bash scripts/demo-complete.sh
```

## Development

### Code Style

- Indentation: 2 spaces
- Quotes: Double quotes
- Semicolons: Always
- Line width: 80 characters

### Type Safety

100% type-safe with TypeScript strict mode.

```bash
bun run type-check
```

### Linting

```bash
bun run lint
bun run format
bun run check
```

## License

MIT
