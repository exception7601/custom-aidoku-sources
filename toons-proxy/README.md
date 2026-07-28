# Toons Proxy

Proxy server for ToonLivre API with automatic encryption fallback.

## Installation

```bash
bun install
```

## Usage

```bash
# Development
bun run dev

# Production
PORT=4000 bun run src/index.ts

# Tests
bun test
```

## Endpoints

### Main API
- `GET /health` - Server status
- `GET /api/releases?page=1&limit=48` - Latest releases
- `GET /api/search?q=term&page=1` - Search manga
- `GET /api/manga/:id` - Manga details
- `GET /api/manga/:id/reader` - Chapter list
- `GET /api/manga/:id/chapters/:chapterId` - Chapter pages

### Control
- `GET /api/encryption/status` - Encryption status
- `POST /api/encryption/toggle` - Toggle encryption on/off
- `POST /api/cache/clear` - Clear cache
- `GET /api/logs` - View access logs
- `DELETE /api/logs` - Clear logs

## Structure

```
toons-proxy/
├── src/
│   ├── index.ts           # HTTP server
│   ├── toonlivre-api.ts   # API client with fallback
│   ├── logger.ts          # Logging system
│   ├── token-manager.ts   # Token management
│   └── token-server.ts    # Token-server client
├── tests/
│   ├── api.test.ts        # Type tests
│   ├── live.test.ts       # Integration tests
│   └── fallback.test.ts   # Fallback tests
└── scripts/               # Helper scripts
```

## Environment Variables

```bash
PORT=4000                                    # Server port
TOKEN_SERVER_HOST=http://localhost:3001      # Token-server (fallback)
```

## Docker

```bash
docker build -t toons-proxy .
docker run -p 4000:4000 toons-proxy
```

## How It Works

1. **Direct access** - Tries without encryption (current mode)
2. **Automatic detection** - If 401/403 error received, activates encryption
3. **Fallback** - Connects to token-server and uses Rabbit encryption
4. **Periodic verification** - Tests for changes every 5 minutes

## Monitoring

```bash
# Status
curl http://localhost:4000/health | jq

# Logs
curl http://localhost:4000/api/logs?limit=50 | jq

# Statistics
curl http://localhost:4000/api/logs/stats | jq
```

## Version

2.0.0
