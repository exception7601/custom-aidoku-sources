# Toons Total Proxy

JSON proxy server for the Toons Total (ToonLivre) API with simplified REST endpoints.

## Features

- Transparent proxy for ToonLivre endpoints
- Automatic token management via Token Server
- Smart caching with configurable TTL
- Complete TypeScript types
- Tests (mocked and live integration)
- Docker ready with multi-stage build
- Linter and Formatter with Biome

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
bun run test
bun run test:live
bun run lint
bun run format
bun run type-check
```

### Docker

```bash
docker-compose up -d
docker-compose logs -f proxy
docker-compose down
```

## Endpoints

### Health Check
```
GET /health
```

### Releases
```
GET /api/releases?page=1&limit=48
```

### Search
```
GET /api/search?q=term&page=1&limit=24
```

### Manga Details
```
GET /api/manga/:id
```

### Manga Reader
```
GET /api/manga/:id/reader
```

### Manga by Slug
```
GET /api/manga-by-slug/:slug
```

### Chapter Details
```
GET /api/manga/:mangaId/chapters/:chapterId
```

## Environment Variables

```bash
PORT=3000
NODE_ENV=production
TOKEN_SERVER_HOST=https://toons.4nd.xyz
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
│   ├── index.ts
│   ├── token-server.ts
│   └── toonlivre-api.ts
├── tests/
│   ├── api.test.ts
│   └── types.test.ts
├── Dockerfile
├── docker-compose.yml
├── package.json
├── tsconfig.json
└── biome.json
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

### Port Already in Use

```bash
PORT=3001 bun run start
```

### Type Errors

```bash
bun run type-check
```

### Connection Error

Check token server:
```bash
curl https://toons.4nd.xyz/health
```

## License

MIT
