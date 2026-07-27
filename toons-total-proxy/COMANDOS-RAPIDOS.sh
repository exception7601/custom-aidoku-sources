#!/bin/bash
# Comandos Rápidos - Toons Total Proxy v2.0.0

# ============================================
# DESENVOLVIMENTO
# ============================================

# Iniciar servidor em desenvolvimento
bun run dev

# Rodar todos os testes
bun test

# Rodar testes específicos
bun test tests/fallback.test.ts
bun test tests/live.test.ts

# Demo completo
bash scripts/demo-complete.sh

# ============================================
# PRODUÇÃO
# ============================================

# Iniciar em produção
NODE_ENV=production PORT=4000 bun run src/index.ts

# Com token-server configurado (fallback)
TOKEN_SERVER_HOST=http://localhost:3001 PORT=4000 bun run src/index.ts

# ============================================
# MONITORAMENTO
# ============================================

# Ver logs (últimos 50)
curl -s "http://localhost:4001/api/logs?limit=50" | jq '.data[] | {method, path, status, responseTime, ip}'

# Estatísticas
curl -s "http://localhost:4001/api/logs/stats" | jq '.data'

# Status de criptografia
curl -s "http://localhost:4001/health" | jq '.encryption'

# Status detalhado
curl -s "http://localhost:4001/api/encryption/status" | jq

# ============================================
# CONTROLE
# ============================================

# Ativar modo criptografado manualmente
curl -X POST "http://localhost:4001/api/encryption/toggle" \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'

# Desativar modo criptografado
curl -X POST "http://localhost:4001/api/encryption/toggle" \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'

# Limpar cache
curl -X POST "http://localhost:4001/api/cache/clear"

# Limpar logs
curl -X DELETE "http://localhost:4001/api/logs"

# ============================================
# TESTES DE API
# ============================================

# Releases
curl -s "http://localhost:4001/api/releases?page=1&limit=10" | jq

# Buscar
curl -s "http://localhost:4001/api/search?q=demon&page=1" | jq

# Manga por slug
curl -s "http://localhost:4001/api/manga-by-slug/contos-de-demonios-e-deuses" | jq

# Manga por ID
curl -s "http://localhost:4001/api/manga/obra-dbbabf0f" | jq

# Reader (lista de capítulos)
curl -s "http://localhost:4001/api/manga/obra-dbbabf0f/reader" | jq

# Capítulo com imagens
curl -s "http://localhost:4001/api/manga/obra-dbbabf0f/chapters/cap-01" | jq

# ============================================
# DOCKER (OPCIONAL)
# ============================================

# Build
docker build -t toons-total-proxy .

# Run
docker run -d \
  --name toons-proxy \
  -p 4000:4000 \
  -e PORT=4000 \
  -e TOKEN_SERVER_HOST=http://token-server:3001 \
  toons-total-proxy

# Logs
docker logs -f toons-proxy

# Stop
docker stop toons-proxy && docker rm toons-proxy
