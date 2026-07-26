# Start Here

Welcome to Toons Total Proxy.

## What is this?

A JSON REST proxy server for ToonLivre manga site with automatic token management and simplified endpoints.

## Quick Setup (5 minutes)

### 1. Start the server

```bash
cd toons-total-proxy
docker-compose up -d
```

### 2. Test it

```bash
curl http://localhost:3000/health | jq
```

Expected response:
```json
{
  "status": "ok",
  "timestamp": "2026-07-26T23:46:19.899Z"
}
```

### 3. Try an endpoint

```bash
curl "http://localhost:3000/api/releases?limit=3" | jq
```

## Available Endpoints

- GET /health - Health check
- GET /api/releases - Trending mangas
- GET /api/search?q=term - Search mangas
- GET /api/manga/:id - Manga details
- GET /api/manga/:id/reader - Manga with chapters

## Next Steps

- Read README.md for full documentation
- Check SETUP_QUICK.md for 5 minute setup

## Stop Server

```bash
docker-compose down
```
