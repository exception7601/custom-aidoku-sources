# Quick Setup

Get the proxy running in 5 minutes.

## Prerequisites

- Docker and Docker Compose installed
- Port 3000 available
- Internet connection

## Step 1: Navigate to Project

```bash
cd /Users/anderson/Developer/custom-aidoku-sources/toons-total-proxy
```

## Step 2: Start Docker Container

```bash
docker-compose up -d
```

Wait for the container to start (5-10 seconds).

## Step 3: Verify Health

```bash
curl http://localhost:3000/health | jq
```

Expected output:
```json
{
  "status": "ok",
  "timestamp": "2026-07-26T23:46:25.943Z"
}
```

## Step 4: Test an Endpoint

```bash
curl "http://localhost:3000/api/releases?page=1&limit=3" | jq
```

You should receive a list of trending mangas.

## Step 5: Stop Server

```bash
docker-compose down
```

## Troubleshooting

### Port already in use

Change the port in docker-compose.yml:
```yaml
ports:
  - "3001:3000"
```

### Container won't start

Check logs:
```bash
docker-compose logs proxy
```

### No response from server

Ensure container is running:
```bash
docker ps
```

Should show toons-total-proxy-proxy-1 as running.
