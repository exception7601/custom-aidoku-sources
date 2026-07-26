#!/bin/bash
set -e

echo "[env] Creating .env.example..."

cat > .env.example << 'EOF'
# Token Server Configuration
TOKEN_SERVER_HOST=https://toons.4nd.xyz

# Server Configuration
PORT=3000
NODE_ENV=production
EOF

echo "[env] ✓ Created .env.example"
echo ""
echo "To use, copy to .env:"
echo "  cp .env.example .env"
