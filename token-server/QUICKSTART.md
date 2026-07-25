# 🚀 Quick Start

## Início Rápido (5 minutos)

### 1. Setup

```bash
cd token-server
chmod +x setup.sh
./setup.sh
```

### 2. Inicie o servidor

```bash
bun run dev
```

### 3. Teste

Em outro terminal:

```bash
# Testes básicos
bun run test

# Teste com API real do ToonLivre
bun run test:live
```

---

## 🐳 Docker Quick Start

```bash
# Build e start
docker compose up -d

# Logs
docker compose logs -f

# Testes
bun run test

# Stop
docker compose down
```

---

## 📝 Exemplo de Uso

### cURL

```bash
curl -X POST http://localhost:3000/api/tokens \
  -H "Content-Type: application/json" \
  -d '{"url": "https://toonlivre.tv/api/manga/solo-leveling/chapters"}'
```

### JavaScript/TypeScript

```typescript
const response = await fetch('http://localhost:3000/api/tokens', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    url: 'https://toonlivre.tv/api/manga/solo-leveling/chapters'
  })
})

const data = await response.json()
console.log(data.headers)
// {
//   "x-toon-signature": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
//   "x-toon-verify": "l1uovawq7na1z3bfpnbt4"
// }
```

### Rust (Aidoku)

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct TokenRequest {
    url: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    headers: std::collections::HashMap<String, String>,
}

async fn get_tokens(url: &str) -> Result<TokenResponse> {
    let response = reqwest::Client::new()
        .post("http://token-server:3000/api/tokens")
        .json(&TokenRequest { url: url.to_string() })
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;
    
    Ok(response)
}
```

---

## 🧪 Scripts de Teste

```bash
# Testes de integração
bun run test

# Teste live com API real
bun run test:live

# Docker + testes
bun run docker:test
```

---

## 📚 Documentação Completa

Veja [README.md](README.md) para documentação completa.
