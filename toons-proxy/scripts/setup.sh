#!/bin/bash
set -e

echo "[setup] Installing dependencies..."
bun install

echo "[setup] Running type check..."
bun run type-check

echo "[setup] Running linter..."
bun run check

echo "[setup] Running tests..."
bun run test

echo "[setup] ✓ All checks passed!"
echo ""
echo "Next steps:"
echo "  bun run dev       - Start development server"
echo "  bun run start     - Start production server"
echo "  bun run test:live - Run live integration tests"
echo "  bun run docker:compose:up - Start with Docker Compose"
