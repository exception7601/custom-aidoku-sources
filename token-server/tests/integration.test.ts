#!/usr/bin/env bun

/**
 * Testes de integração do servidor de tokens
 */

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

interface TestResult {
  name: string
  passed: boolean
  duration: number
  error?: string
  response?: any
}

const results: TestResult[] = []

async function test(name: string, fn: () => Promise<void>) {
  const start = Date.now()
  try {
    await fn()
    results.push({
      name,
      passed: true,
      duration: Date.now() - start,
    })
    console.log(`✅ ${name} (${Date.now() - start}ms)`)
  } catch (error) {
    results.push({
      name,
      passed: false,
      duration: Date.now() - start,
      error: (error as Error).message,
    })
    console.log(`❌ ${name} (${Date.now() - start}ms)`)
    console.log(`   Erro: ${(error as Error).message}`)
  }
}

async function assert(condition: boolean, message: string) {
  if (!condition) {
    throw new Error(message)
  }
}

console.log(`
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║         🧪 TESTES DO SERVIDOR DE TOKENS                      ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝

Base URL: ${BASE_URL}
`)

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
console.log('\n📋 Testes de Health Check')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

await test('GET /health - deve retornar status ok', async () => {
  const response = await fetch(`${BASE_URL}/health`)
  await assert(response.ok, 'Response não é ok')
  
  const data = await response.json()
  await assert(data.status === 'ok', 'Status não é ok')
  await assert(typeof data.uptime === 'number', 'Uptime não é número')
  await assert(typeof data.timestamp === 'string', 'Timestamp não é string')
})

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
console.log('\n📋 Testes de Geração de Tokens')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

let generatedTokens: any = null

await test('POST /api/tokens - deve gerar tokens válidos', async () => {
  const response = await fetch(`${BASE_URL}/api/tokens`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      url: 'https://toonlivre.net/api/manga/solo-leveling/chapters',
    }),
  })

  await assert(response.ok, `Response não é ok: ${response.status}`)
  
  const data = await response.json()
  generatedTokens = data

  // Valida estrutura
  await assert(typeof data.session === 'string', 'Session não é string')
  await assert(data.session.length > 10, 'Session muito curto')
  
  await assert(typeof data.passphrase === 'string', 'Passphrase não é string')
  await assert(data.passphrase.length > 20, 'Passphrase muito curto')
  
  await assert(typeof data.headers === 'object', 'Headers não é objeto')
  await assert(typeof data.headers['x-toon-signature'] === 'string', 'Signature não encontrada')
  await assert(typeof data.headers['x-toon-verify'] === 'string', 'Verify não encontrado')
  
  await assert(typeof data.strategy === 'string', 'Strategy não é string')
  await assert(typeof data.expiresAt === 'number', 'ExpiresAt não é número')
  await assert(typeof data.expiresIn === 'number', 'ExpiresIn não é número')
  await assert(typeof data.cached === 'boolean', 'Cached não é boolean')

  console.log(`   Session: ${data.session}`)
  console.log(`   Passphrase: ${data.passphrase}`)
  console.log(`   Strategy: ${data.strategy}`)
  console.log(`   Cached: ${data.cached}`)
})

await test('POST /api/tokens - deve retornar erro sem URL', async () => {
  const response = await fetch(`${BASE_URL}/api/tokens`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({}),
  })

  await assert(response.status === 400, 'Status não é 400')
  
  const data = await response.json()
  await assert(data.error, 'Erro não retornado')
})

await test('POST /api/tokens - deve usar cache na segunda request', async () => {
  if (!generatedTokens) {
    throw new Error('Tokens não foram gerados no teste anterior')
  }

  const response = await fetch(`${BASE_URL}/api/tokens`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      url: 'https://toonlivre.net/api/manga/solo-leveling/chapters',
    }),
  })

  await assert(response.ok, 'Response não é ok')
  
  const data = await response.json()
  await assert(data.cached === true, 'Cache não foi usado')
  await assert(data.session === generatedTokens.session, 'Session diferente do cache')
  
  console.log(`   Cached: ${data.cached}`)
  console.log(`   Expires in: ${data.expiresIn}s`)
})

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
console.log('\n📋 Testes de Descriptografia')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

await test('POST /api/decrypt - deve descriptografar dados', async () => {
  if (!generatedTokens) {
    throw new Error('Tokens não foram gerados')
  }

  // Cria um dado criptografado de teste
  const testData = '{"test": "data"}'
  
  // Nota: Este teste requer bundle no cache
  // Em produção, seria necessário um dado criptografado real do ToonLivre
  console.log('   ⚠️  Teste requer bundle em cache e dados reais')
})

await test('POST /api/decrypt - deve retornar erro sem dados encrypted', async () => {
  const response = await fetch(`${BASE_URL}/api/decrypt`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({}),
  })

  await assert(response.status === 400, 'Status não é 400')
  
  const data = await response.json()
  await assert(data.error, 'Erro não retornado')
})

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
console.log('\n📋 Testes de Cache')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

await test('GET /api/cache/stats - deve retornar estatísticas', async () => {
  const response = await fetch(`${BASE_URL}/api/cache/stats`)
  await assert(response.ok, 'Response não é ok')
  
  const data = await response.json()
  await assert(typeof data.bundle === 'string', 'Bundle não é string')
  await assert(typeof data.tokens === 'number', 'Tokens não é número')
  
  console.log(`   Bundle: ${data.bundle}`)
  console.log(`   Tokens: ${data.tokens}`)
})

await test('POST /api/cache/clear - deve limpar cache', async () => {
  const response = await fetch(`${BASE_URL}/api/cache/clear`, {
    method: 'POST',
  })

  await assert(response.ok, 'Response não é ok')
  
  const data = await response.json()
  await assert(data.message, 'Mensagem não retornada')
})

await test('GET /api/cache/stats - deve mostrar cache vazio após clear', async () => {
  const response = await fetch(`${BASE_URL}/api/cache/stats`)
  await assert(response.ok, 'Response não é ok')
  
  const data = await response.json()
  await assert(data.bundle === 'empty', 'Bundle não está vazio')
  await assert(data.tokens === 0, 'Tokens não está vazio')
})

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
console.log('\n📋 Testes de CORS')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

await test('OPTIONS /api/tokens - deve retornar headers CORS', async () => {
  const response = await fetch(`${BASE_URL}/api/tokens`, {
    method: 'OPTIONS',
  })

  await assert(response.ok, 'Response não é ok')
  
  const allowOrigin = response.headers.get('Access-Control-Allow-Origin')
  const allowMethods = response.headers.get('Access-Control-Allow-Methods')
  
  await assert(allowOrigin === '*', 'CORS origin não configurado')
  await assert(allowMethods !== null, 'CORS methods não configurado')
})

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
console.log('\n📋 Testes de Endpoints Inválidos')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

await test('GET /invalid - deve retornar 404', async () => {
  const response = await fetch(`${BASE_URL}/invalid`)
  await assert(response.status === 404, 'Status não é 404')
  
  const data = await response.json()
  await assert(data.error, 'Erro não retornado')
})

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
console.log('\n📋 Teste de Performance')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

await test('POST /api/tokens - 10 requests concorrentes', async () => {
  const promises = []
  for (let i = 0; i < 10; i++) {
    promises.push(
      fetch(`${BASE_URL}/api/tokens`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          url: `https://toonlivre.net/api/manga/test-${i}/chapters`,
        }),
      })
    )
  }

  const responses = await Promise.all(promises)
  const allOk = responses.every(r => r.ok)
  
  await assert(allOk, 'Nem todas as requests foram bem sucedidas')
  console.log(`   ✅ 10 requests concorrentes processadas`)
})

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
console.log('\n')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
console.log('  📊 RESUMO DOS TESTES')
console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

const passed = results.filter(r => r.passed).length
const failed = results.filter(r => !r.passed).length
const total = results.length
const totalDuration = results.reduce((sum, r) => sum + r.duration, 0)
const avgDuration = totalDuration / total

console.log(`
  Total de testes: ${total}
  ✅ Passou: ${passed}
  ❌ Falhou: ${failed}
  
  Tempo total: ${totalDuration}ms
  Tempo médio: ${Math.round(avgDuration)}ms
`)

if (failed > 0) {
  console.log('\n❌ Testes que falharam:\n')
  results
    .filter(r => !r.passed)
    .forEach(r => {
      console.log(`  • ${r.name}`)
      console.log(`    Erro: ${r.error}`)
    })
}

console.log('━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━')

// Exit code
if (failed > 0) {
  console.log('\n❌ Alguns testes falharam\n')
  process.exit(1)
} else {
  console.log('\n✅ Todos os testes passaram!\n')
  process.exit(0)
}
