# ToonLivre Token Server

Servidor HTTP para geração de tokens e descriptografia de dados do ToonLivre.

## 🚀 Features

- ✅ Geração automática de tokens (session, passphrase, headers)
- ✅ Descriptografia de dados criptografados
- ✅ Cache inteligente (bundle: 10 min, tokens: 25 seg)
- ✅ Detecção automática de estratégia (seed-jwt)
- ✅ Docker pronto para produção
- ✅ Health checks
- ✅ CORS habilitado

---

## 📦 Instalação

### Desenvolvimento Local

```bash
cd token-server
bun install
bun run dev
```

### Docker (Recomendado)

```bash
cd token-server
docker compose up -d
```

---

## 🔌 API Endpoints

### 1. POST `/api/tokens` - Gera Tokens

**Request:**
```bash
curl -X POST http://localhost:3000/api/tokens \
  -H "Content-Type: application/json" \
  -d '{"url": "https://toonlivre.tv/api/manga/solo-leveling/chapters"}'
```

**Response:**
```json
{
  "session": "l1uovawq7na1z3bfpnbt4",
  "passphrase": "Vortex-Blade-Nexus4b97f079c",
  "headers": {
    "x-toon-signature": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "x-toon-verify": "l1uovawq7na1z3bfpnbt4"
  },
  "strategy": "seed-jwt",
  "expiresAt": 1706200825000,
  "cached": false,
  "expiresIn": 25
}
```

**Cache:**
- ✅ Bundle: 10 minutos
- ✅ Tokens: 25 segundos (válido por 25s)

---

### 2. POST `/api/decrypt` - Descriptografa Dados

**Request:**
```bash
curl -X POST http://localhost:3000/api/decrypt \
  -H "Content-Type: application/json" \
  -d '{
    "encrypted": "U2FsdGVkX1...",
    "passphrase": "Vortex-Blade-Nexus4b97f079c"
  }'
```

**Response:**
```json
{
  "decrypted": "{\"chapters\": [...]}"
}
```

---

### 3. GET `/api/cache/stats` - Estatísticas do Cache

**Request:**
```bash
curl http://localhost:3000/api/cache/stats
```

**Response:**
```json
{
  "bundle": "cached",
  "tokens": 3
}
```

---

### 4. POST `/api/cache/clear` - Limpa Cache

```bash
curl -X POST http://localhost:3000/api/cache/clear
```

---

### 5. GET `/health` - Health Check

```bash
curl http://localhost:3000/health
```

**Response:**
```json
{
  "status": "ok",
  "uptime": 123.456,
  "timestamp": "2026-07-25T15:52:16.026Z"
}
```

---

## 🦀 Integração com Aidoku (Rust)

### Exemplo de Uso

```rust
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
struct TokenRequest {
    url: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    session: String,
    passphrase: String,
    headers: std::collections::HashMap<String, String>,
    strategy: String,
    #[serde(rename = "expiresIn")]
    expires_in: u64,
}

#[derive(Serialize)]
struct DecryptRequest {
    encrypted: String,
    passphrase: Option<String>,
}

#[derive(Deserialize)]
struct DecryptResponse {
    decrypted: String,
}

async fn get_chapter_data(chapter_url: &str) -> Result<Value> {
    // 1. Busca tokens do servidor
    let token_response = reqwest::Client::new()
        .post("http://token-server:3000/api/tokens")
        .json(&TokenRequest {
            url: chapter_url.to_string(),
        })
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;

    // 2. Faz request para API do ToonLivre
    let response = reqwest::Client::new()
        .get(chapter_url)
        .header("x-toon-signature", &token_response.headers["x-toon-signature"])
        .header("x-toon-verify", &token_response.headers["x-toon-verify"])
        .header("User-Agent", "Mozilla/5.0...")
        .send()
        .await?;

    // 3. Verifica se está criptografado
    let datakey = response.headers().get("x-toon-datakey");
    
    if let Some(key) = datakey {
        let json: Value = response.json().await?;
        let encrypted = json[key.to_str()?].as_str().ok_or("Invalid encrypted data")?;

        // 4. Descriptografa
        let decrypt_response = reqwest::Client::new()
            .post("http://token-server:3000/api/decrypt")
            .json(&DecryptRequest {
                encrypted: encrypted.to_string(),
                passphrase: Some(token_response.passphrase),
            })
            .send()
            .await?
            .json::<DecryptResponse>()
            .await?;

        let data: Value = serde_json::from_str(&decrypt_response.decrypted)?;
        Ok(data)
    } else {
        // Dados não criptografados
        let data = response.json().await?;
        Ok(data)
    }
}

// Uso
#[tokio::main]
async fn main() -> Result<()> {
    let data = get_chapter_data("https://toonlivre.tv/api/manga/solo-leveling/chapters").await?;
    println!("{:#?}", data);
    Ok(())
}
```

---

## 🐳 Docker

### Build

```bash
docker compose build
```

### Start

```bash
docker compose up -d
```

### Logs

```bash
docker compose logs -f
```

### Stop

```bash
docker compose down
```

### Reiniciar

```bash
docker compose restart
```

---

## ⚙️ Variáveis de Ambiente

| Variável | Padrão | Descrição |
|----------|--------|-----------|
| `PORT` | `3000` | Porta do servidor |
| `NODE_ENV` | `production` | Ambiente |

---

## 📊 Cache TTL

| Item | TTL | Motivo |
|------|-----|--------|
| **Bundle** | 10 minutos | Bundle muda raramente |
| **Tokens** | 25 segundos | JWT do seed expira em ~25 minutos, mas renovamos a cada 25s para garantir |

---

## 🔒 Segurança

### Melhorias Recomendadas

1. **Rate Limiting**
   ```typescript
   // Adicionar rate limit por IP
   const rateLimiter = new Map<string, number[]>()
   
   function checkRateLimit(ip: string): boolean {
     const now = Date.now()
     const requests = rateLimiter.get(ip) || []
     const recent = requests.filter(t => now - t < 60000) // últimos 60s
     
     if (recent.length >= 60) return false // máximo 60 req/min
     
     recent.push(now)
     rateLimiter.set(ip, recent)
     return true
   }
   ```

2. **API Key**
   ```typescript
   const API_KEY = process.env.API_KEY || 'your-secret-key'
   
   if (req.headers.get('x-api-key') !== API_KEY) {
     return Response.json({ error: 'Unauthorized' }, { status: 401 })
   }
   ```

3. **HTTPS**
   - Use reverse proxy (nginx/caddy) com SSL
   - Let's Encrypt para certificados gratuitos

4. **Firewall**
   - Restrinja acesso apenas ao Aidoku
   - Use Docker networks privadas

---

## 🚀 Deploy em Produção

### Docker Compose + Caddy (HTTPS automático)

```yaml
version: '3.8'

services:
  token-server:
    build: .
    container_name: toonlivre-token-server
    restart: unless-stopped
    networks:
      - internal
    environment:
      - PORT=3000

  caddy:
    image: caddy:2-alpine
    container_name: caddy-proxy
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
      - caddy_data:/data
      - caddy_config:/config
    networks:
      - internal

networks:
  internal:
    driver: bridge

volumes:
  caddy_data:
  caddy_config:
```

**Caddyfile:**
```
tokens.example.com {
    reverse_proxy token-server:3000
}
```

---

## 📈 Monitoramento

### Prometheus Metrics (Opcional)

```typescript
let requestCount = 0
let errorCount = 0
let cacheHits = 0
let cacheMisses = 0

// GET /metrics
if (url.pathname === '/metrics') {
  return new Response(`
# HELP requests_total Total number of requests
# TYPE requests_total counter
requests_total ${requestCount}

# HELP errors_total Total number of errors
# TYPE errors_total counter
errors_total ${errorCount}

# HELP cache_hits_total Total number of cache hits
# TYPE cache_hits_total counter
cache_hits_total ${cacheHits}

# HELP cache_misses_total Total number of cache misses
# TYPE cache_misses_total counter
cache_misses_total ${cacheMisses}
  `, {
    headers: { 'Content-Type': 'text/plain' }
  })
}
```

---

## 🧪 Testes

### Teste Local

```bash
# 1. Start servidor
bun run dev

# 2. Gera tokens
curl -X POST http://localhost:3000/api/tokens \
  -H "Content-Type: application/json" \
  -d '{"url": "https://toonlivre.tv/api/manga/solo-leveling/chapters"}'

# 3. Verifica cache
curl http://localhost:3000/api/cache/stats
```

### Teste Docker

```bash
docker compose up -d
docker compose logs -f
curl http://localhost:3000/health
```

---

## 📝 Logs

Logs são salvos em JSON format com rotação automática:
- Máximo: 10MB por arquivo
- Arquivos mantidos: 3

---

## ✅ Status

- ✅ Servidor HTTP funcionando
- ✅ Cache implementado
- ✅ Docker pronto
- ✅ Health checks
- ✅ CORS habilitado
- ✅ Logs estruturados
- ✅ Pronto para produção

---

## 🎯 Roadmap

- [ ] Rate limiting
- [ ] API key authentication
- [ ] Prometheus metrics
- [ ] Redis para cache distribuído
- [ ] Testes automatizados
- [ ] CI/CD pipeline

---

## 📄 Licença

MIT
