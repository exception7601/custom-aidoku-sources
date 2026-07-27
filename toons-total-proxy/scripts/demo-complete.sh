#!/bin/bash

echo "========================================"
echo "  TOONS TOTAL PROXY - DEMO COMPLETO"
echo "========================================"
echo ""

BASE_URL="${BASE_URL:-http://localhost:4001}"

echo "🔍 1. Health Check"
curl -s "$BASE_URL/health" | jq '{status, encryption}'
echo ""

echo "🔍 2. Status de Criptografia"
curl -s "$BASE_URL/api/encryption/status" | jq '.data'
echo ""

echo "📚 3. Buscar Releases (2 primeiros)"
curl -s "$BASE_URL/api/releases?page=1&limit=2" | jq '{success, count: (.data.mangas | length)}'
echo ""

echo "🔎 4. Buscar Manga"
curl -s "$BASE_URL/api/search?q=demon&page=1&limit=2" | jq '{success, count: (.data.mangas | length)}'
echo ""

echo "📖 5. Detalhes do Manga por Slug"
curl -s "$BASE_URL/api/manga-by-slug/contos-de-demonios-e-deuses" | jq '{success, title: .data.title, id: .data.id}'
echo ""

echo "📊 6. Estatísticas de Logs"
curl -s "$BASE_URL/api/logs/stats" | jq '.data'
echo ""

echo "📋 7. Últimos 3 Logs"
curl -s "$BASE_URL/api/logs?limit=3" | jq '.data[] | {method, path, status, responseTime}'
echo ""

echo "🔄 8. Alternar para Modo Criptografado"
curl -s -X POST "$BASE_URL/api/encryption/toggle" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}' | jq '.data'
echo ""

echo "✅ 9. Verificar Mudança"
curl -s "$BASE_URL/api/encryption/status" | jq '.data'
echo ""

echo "🔄 10. Voltar para Modo Direto"
curl -s -X POST "$BASE_URL/api/encryption/toggle" \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}' | jq '.data'
echo ""

echo "🗑️  11. Limpar Cache"
curl -s -X POST "$BASE_URL/api/cache/clear" | jq '{success, message}'
echo ""

echo "========================================"
echo "  ✅ DEMO COMPLETO FINALIZADO"
echo "========================================"
