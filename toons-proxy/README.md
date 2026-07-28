# Toons Proxy

Proxy server para a API do ToonLivre com fallback automático de criptografia.

## Instalação

```bash
bun install
```

## Uso

```bash
# Desenvolvimento
bun run dev

# Produção
PORT=4000 bun run src/index.ts

# Testes
bun test
```

## Endpoints

### API Principal
- `GET /health` - Status do servidor
- `GET /api/releases?page=1&limit=48` - Lançamentos
- `GET /api/search?q=termo&page=1` - Buscar mangás
- `GET /api/manga/:id` - Detalhes do mangá
- `GET /api/manga/:id/reader` - Lista de capítulos
- `GET /api/manga/:id/chapters/:chapterId` - Páginas do capítulo

### Controle
- `GET /api/encryption/status` - Status da criptografia
- `POST /api/encryption/toggle` - Ativar/desativar criptografia
- `POST /api/cache/clear` - Limpar cache
- `GET /api/logs` - Ver logs de acesso
- `DELETE /api/logs` - Limpar logs

## Estrutura

```
toons-proxy/
├── src/
│   ├── index.ts           # Servidor HTTP
│   ├── toonlivre-api.ts   # Cliente API com fallback
│   ├── logger.ts          # Sistema de logs
│   ├── token-manager.ts   # Gerenciamento de tokens
│   └── token-server.ts    # Cliente token-server
├── tests/
│   ├── api.test.ts        # Testes de tipos
│   ├── live.test.ts       # Testes de integração
│   └── fallback.test.ts   # Testes de fallback
└── scripts/               # Scripts auxiliares
```

## Variáveis de Ambiente

```bash
PORT=4000                                    # Porta do servidor
TOKEN_SERVER_HOST=http://localhost:3001      # Token-server (fallback)
```

## Docker

```bash
docker build -t toons-proxy .
docker run -p 4000:4000 toons-proxy
```

## Como Funciona

1. **Acesso direto** - Tenta sem criptografia (modo atual)
2. **Detecção automática** - Se receber erro 401/403, ativa criptografia
3. **Fallback** - Conecta no token-server e usa criptografia Rabbit
4. **Verificação periódica** - Testa mudanças a cada 5 minutos

## Monitoramento

```bash
# Status
curl http://localhost:4000/health | jq

# Logs
curl http://localhost:4000/api/logs?limit=50 | jq

# Estatísticas
curl http://localhost:4000/api/logs/stats | jq
```

## Versão

2.0.0
