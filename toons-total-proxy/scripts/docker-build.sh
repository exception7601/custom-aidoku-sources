#!/bin/bash
set -e

echo "[docker] Building image..."
docker build -t toons-total-proxy:latest .

echo "[docker] ✓ Build complete!"
echo ""
echo "Run with:"
echo "  docker run -p 3000:3000 toons-total-proxy:latest"
echo ""
echo "Or with docker-compose:"
echo "  docker-compose up -d"
