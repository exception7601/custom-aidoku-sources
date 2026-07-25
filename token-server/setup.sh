#!/bin/bash

# Script de setup para o Token Server

set -e

echo "
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║         🚀 SETUP: TOONLIVRE TOKEN SERVER                     ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
"

# Verifica se Bun está instalado
if ! command -v bun &> /dev/null; then
    echo "❌ Bun não encontrado"
    echo ""
    echo "Instale Bun primeiro:"
    echo "  curl -fsSL https://bun.sh/install | bash"
    exit 1
fi

echo "✅ Bun instalado: $(bun --version)"

# Instala dependências
echo ""
echo "📦 Instalando dependências..."
bun install

echo ""
echo "✅ Dependências instaladas"

# Testa se o servidor inicia
echo ""
echo "🧪 Testando se o servidor inicia..."
timeout 3 bun run src/server.ts &> /dev/null || true

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✅ SETUP COMPLETO"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Próximos passos:"
echo ""
echo "  1. Iniciar servidor:"
echo "     bun run dev"
echo ""
echo "  2. Testar servidor:"
echo "     bun run test"
echo ""
echo "  3. Teste live (API real):"
echo "     bun run test:live"
echo ""
echo "  4. Docker:"
echo "     docker compose up -d"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
