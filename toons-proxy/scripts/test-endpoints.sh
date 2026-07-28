#!/usr/bin/env bash
# Quick test script for all proxy endpoints

set -e

BASE_URL="${BASE_URL:-http://localhost:4001}"

echo "================================"
echo "Toons Total Proxy - Quick Test"
echo "================================"
echo ""
echo "Base URL: $BASE_URL"
echo ""

# Health check
echo "[1/7] Testing health endpoint..."
curl -s "$BASE_URL/health" | jq -r '.status' > /dev/null && echo "✓ Health check passed" || echo "✗ Health check failed"

# Releases
echo "[2/7] Testing releases endpoint..."
curl -s "$BASE_URL/api/releases?page=1&limit=2" | jq -r '.success' | grep -q true && echo "✓ Releases passed" || echo "✗ Releases failed"

# Search
echo "[3/7] Testing search endpoint..."
curl -s "$BASE_URL/api/search?q=demon&page=1&limit=2" | jq -r '.success' | grep -q true && echo "✓ Search passed" || echo "✗ Search failed"

# Manga by slug
echo "[4/7] Testing manga-by-slug endpoint..."
MANGA_DATA=$(curl -s "$BASE_URL/api/manga-by-slug/contos-de-demonios-e-deuses")
echo "$MANGA_DATA" | jq -r '.success' | grep -q true && echo "✓ Manga by slug passed" || echo "✗ Manga by slug failed"
MANGA_ID=$(echo "$MANGA_DATA" | jq -r '.data.id')

# Manga by ID
echo "[5/7] Testing manga by ID endpoint..."
curl -s "$BASE_URL/api/manga/$MANGA_ID" | jq -r '.success' | grep -q true && echo "✓ Manga by ID passed" || echo "✗ Manga by ID failed"

# Reader
echo "[6/7] Testing reader endpoint..."
curl -s "$BASE_URL/api/manga/$MANGA_ID/reader" | jq -r '.success' | grep -q true && echo "✓ Reader passed" || echo "✗ Reader failed"

# Image proxy
echo "[7/7] Testing image proxy endpoint..."
IMAGE_URL="https://cdn.toonlivre.net/covers/obra-0367457a/cover-808cb591a682a161150383fcf3f549c7.webp"
ENCODED_URL=$(echo "$IMAGE_URL" | jq -sRr @uri)
curl -s "$BASE_URL/api/image?url=$ENCODED_URL" -o /tmp/proxy_test.webp
file /tmp/proxy_test.webp | grep -q "Web/P" && echo "✓ Image proxy passed" || echo "✗ Image proxy failed"
rm -f /tmp/proxy_test.webp

echo ""
echo "================================"
echo "All tests completed!"
echo "================================"
