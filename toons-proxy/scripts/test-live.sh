#!/bin/bash
set -e

echo "[test] Starting live integration tests..."
echo ""
echo "Make sure the server is running:"
echo "  bun run dev  (in another terminal)"
echo ""

sleep 2

echo "[test] Running tests against http://localhost:3000"
bun test tests/api.test.ts

echo ""
echo "[test] ✓ Tests complete!"
