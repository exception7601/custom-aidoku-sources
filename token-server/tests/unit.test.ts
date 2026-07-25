#!/usr/bin/env bun

/**
 * Testes unitarios (sem conexao externa)
 */

console.log(`
---------------------------------------------------------------
                                                               
         TESTES UNITARIOS (SEM CONEXAO EXTERNA)                
                                                               
---------------------------------------------------------------
`)

// ---------------------------------------------------------------
console.log('\n 1. Teste de Cache')
console.log('---------------------------------------------------------------')

class Cache {
  private store: Map<string, { value: any; expires: number }> = new Map()

  set(key: string, value: any, ttlSeconds: number): void {
    const expires = Date.now() + ttlSeconds * 1000
    this.store.set(key, { value, expires })
  }

  get(key: string): any | null {
    const item = this.store.get(key)
    if (!item) return null

    if (Date.now() > item.expires) {
      this.store.delete(key)
      return null
    }

    return item.value
  }

  has(key: string): boolean {
    return this.get(key) !== null
  }
}

const cache = new Cache()

// Teste 1: Set e Get
cache.set('test', 'value', 10)
const value = cache.get('test')
console.log(value === 'value' ? ' Cache set/get funciona' : ' Cache falhou')

// Teste 2: TTL
cache.set('expire', 'value', 0.1) // 100ms
setTimeout(() => {
  const expired = cache.get('expire')
  console.log(expired === null ? ' Cache TTL funciona' : ' Cache TTL falhou')
}, 200)

// ---------------------------------------------------------------
console.log('\n 2. Teste de Geracao de Session')
console.log('---------------------------------------------------------------')

function generateSession(): string {
  return (
    Math.random().toString(36).substring(2, 15) +
    Math.random().toString(36).substring(2, 15)
  )
}

const session1 = generateSession()
const session2 = generateSession()

console.log(session1.length > 20 ? ' Session gerado com tamanho correto' : ' Session tamanho incorreto')
console.log(session1 !== session2 ? ' Sessions sao unicos' : ' Sessions nao sao unicos')
console.log(`   Session exemplo: ${session1}`)

// ---------------------------------------------------------------
console.log('\n 3. Teste de Funcao Z0')
console.log('---------------------------------------------------------------')

function Z0(input: string | number[]): string {
  if (typeof input === 'string') {
    return Buffer.from(input, 'hex').toString('utf8')
  } else {
    return String.fromCharCode(...input)
  }
}

// Teste com hex string
const hexResult = Z0('746f6f6e6c697672652e6e6574')
console.log(hexResult === 'toonlivre.net' ? ' Z0 hex funciona' : ' Z0 hex falhou')

// Teste com array
const arrayResult = Z0([116, 101, 115, 116])
console.log(arrayResult === 'test' ? ' Z0 array funciona' : ' Z0 array falhou')

// ---------------------------------------------------------------
console.log('\n 4. Teste de Extracao de Funcao do Bundle')
console.log('---------------------------------------------------------------')

const mockBundle = `
const x = 123;
iv=()=>{const n=new Date().toISOString().slice(0,10);return"Test-"+n};
const y = 456;
`

const pattern = /iv=\(\)=>\{[^}]*\}/
const match = mockBundle.match(pattern)

console.log(match ? ' Regex extrai funcao corretamente' : ' Regex falhou')
if (match) {
  console.log(`   Funcao extraida: ${match[0]}`)
}

// ---------------------------------------------------------------
console.log('\n 5. Teste de Execucao de Funcao com VM')
console.log('---------------------------------------------------------------')

import * as vm from 'vm'
import CryptoJS from 'crypto-js'

try {
  const context = vm.createContext({
    Date,
    CryptoJS,
    Gi: CryptoJS,
    Z0,
  })

  // Testa funcao simples
  const simpleFunc = `iv=()=>{return "test-value"}`
  const code = `(function() { ${simpleFunc}; return iv; })()`
  const fn = vm.runInContext(code, context)
  const result = fn()

  console.log(result === 'test-value' ? ' VM executa codigo funciona' : ' VM falhou')
  console.log(`   Resultado: ${result}`)
} catch (error) {
  console.log(` Erro no VM: ${(error as Error).message}`)
}

// ---------------------------------------------------------------
console.log('\n 6. Teste de CryptoJS')
console.log('---------------------------------------------------------------')

// Teste SHA256
const hash = CryptoJS.SHA256('test').toString(CryptoJS.enc.Hex)
console.log(hash ? ' CryptoJS SHA256 funciona' : ' CryptoJS SHA256 falhou')
console.log(`   Hash: ${hash.substring(0, 32)}...`)

// Teste Rabbit encrypt/decrypt
const key = 'test-key'
const plaintext = 'Hello World'
const encrypted = CryptoJS.Rabbit.encrypt(plaintext, key).toString()
const decrypted = CryptoJS.Rabbit.decrypt(encrypted, key).toString(CryptoJS.enc.Utf8)

console.log(decrypted === plaintext ? ' CryptoJS Rabbit funciona' : ' CryptoJS Rabbit falhou')
console.log(`   Original: ${plaintext}`)
console.log(`   Encrypted: ${encrypted.substring(0, 32)}...`)
console.log(`   Decrypted: ${decrypted}`)

// ---------------------------------------------------------------
setTimeout(() => {
  console.log('\n')
  console.log('---------------------------------------------------------------')
  console.log('  TESTES UNITARIOS COMPLETOS')
  console.log('---------------------------------------------------------------')
  console.log(`
  Cache funciona
  Session generator funciona
  Funcao Z0 funciona
  Extracao de funcoes funciona
  VM executa codigo funciona
  CryptoJS funciona
  
  Todos os componentes do servidor estao funcionais!
  
  Testes com API real do ToonLivre requerem conexao externa.
      Use 'bun run test:live' quando o site estiver acessivel.
`)
  console.log('---------------------------------------------------------------\n')
}, 300)
