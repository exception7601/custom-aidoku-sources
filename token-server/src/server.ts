import CryptoJS from 'crypto-js'
import * as vm from 'vm'

/**
 * Simple in-memory cache with TTL
 */
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

  delete(key: string): void {
    this.store.delete(key)
  }

  clear(): void {
    this.store.clear()
  }

  // Periodic cleanup of expired items
  cleanup(): void {
    const now = Date.now()
    for (const [key, item] of this.store.entries()) {
      if (now > item.expires) {
        this.store.delete(key)
      }
    }
  }
}

/**
 * Z0 function to decode strings from bundle
 */
function Z0(input: string | number[]): string {
  if (typeof input === 'string') {
    return Buffer.from(input, 'hex').toString('utf8')
  } else {
    return String.fromCharCode(...input)
  }
}

/**
 * Simple logger
 */
function log(level: 'info' | 'error' | 'cache' | 'decrypt' | 'access', message: string) {
  const timestamp = new Date().toISOString()
  console.log(`[${timestamp}] [${level.toUpperCase()}] ${message}`)
}

/**
 * Token executor
 */
class TokenExecutor {
  private bundleCode: string
  private context: any

  constructor(bundleCode: string) {
    this.bundleCode = bundleCode
    this.context = vm.createContext({
      Date,
      Math,
      CryptoJS,
      Gi: CryptoJS,
      To: CryptoJS,
      Wi: CryptoJS,
      Z0,
    })
  }

  /**
   * Generate session ID
   */
  generateSession(): string {
    return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15)
  }

  /**
   * Extract and execute passphrase function
   */
  generatePassphrase(): string {
    // Search for passphrase function (iv, nv, sv, etc)
    const patterns = [
      /([a-z]{2})=\(\)=>\{[^}]*SHA256[^}]*slice\(0,8\)[^}]*\}/,
      /([a-z]{2})=\(\)=>\{[^}]*MD5[^}]*substring\(0,8\)[^}]*\}/,
    ]

    for (const pattern of patterns) {
      const match = this.bundleCode.match(pattern)
      if (match) {
        const funcName = match[1]

        // Extract complete function
        const start = this.bundleCode.indexOf(`${funcName}=()=>{`)
        if (start === -1) continue

        let depth = 0
        let inString = false
        let stringChar = null
        let end = start + `${funcName}=()=>`.length

        for (let i = end; i < this.bundleCode.length && i < start + 1000; i++) {
          const char = this.bundleCode[i]
          const prev = this.bundleCode[i - 1]

          if ((char === '"' || char === "'") && prev !== '\\') {
            if (!inString) {
              inString = true
              stringChar = char
            } else if (char === stringChar) {
              inString = false
              stringChar = null
            }
          }

          if (!inString) {
            if (char === '{') depth++
            if (char === '}') {
              depth--
              if (depth === 0) {
                end = i + 1
                break
              }
            }
          }
        }

        const funcCode = this.bundleCode.substring(start, end)

        // Execute function
        try {
          const code = `(function() { ${funcCode}; return ${funcName}; })()`
          const fn = vm.runInContext(code, this.context)
          return fn()
        } catch (error) {
          log('error', `Error executing ${funcName}: ${(error as Error).message}`)
          continue
        }
      }
    }

    throw new Error('Passphrase function not found in bundle')
  }

  /**
   * Fetch seed JWT from server
   */
  async fetchSeedJWT(baseUrl: string = 'https://toonlivre.net'): Promise<string> {
    try {
      const response = await fetch(`${baseUrl}/api/seed`, {
        credentials: 'include',
        cache: 'no-store',
        headers: {
          'User-Agent':
            'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
          Accept: 'application/json, text/plain, */*',
          'Accept-Language': 'pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7',
          'Accept-Encoding': 'gzip, deflate, br',
          Referer: baseUrl,
          Origin: baseUrl,
        },
      })

      if (!response.ok) {
        throw new Error(`Seed endpoint returned ${response.status}`)
      }

      const data = await response.json()

      if (!data?.token) {
        throw new Error('Token not found in response')
      }

      return data.token
    } catch (error) {
      throw new Error(`Failed to fetch seed JWT: ${(error as Error).message}`, {
        cause: error,
      })
    }
  }

  /**
   * Detect bundle strategy
   */
  detectStrategy(): 'seed-jwt' | 'hash' {
    if (this.bundleCode.includes('/api/seed')) {
      return 'seed-jwt'
    }
    return 'hash'
  }

  /**
   * Generate headers automatically
   */
  async generateHeaders(_url: string): Promise<Record<string, string>> {
    const strategy = this.detectStrategy()
    const session = this.generateSession()

    if (strategy === 'seed-jwt') {
      const jwt = await this.fetchSeedJWT()
      return {
        'x-toon-signature': jwt,
        'x-toon-verify': session,
      }
    } else {
      // For old strategy, would need to implement generateSignature
      // For now, assumes seed-jwt
      throw new Error('Hash strategy not implemented in server')
    }
  }

  /**
   * Decrypt data
   */
  decrypt(encrypted: string, passphrase?: string): string {
    const key = passphrase || this.generatePassphrase()
    return vm.runInContext(
      `CryptoJS.Rabbit.decrypt("${encrypted}", "${key}").toString(CryptoJS.enc.Utf8)`,
      this.context
    )
  }
}

/**
 * HTTP Server
 */
const bundleCache = new Cache()
const tokenCache = new Cache()

// Periodic cache cleanup (every 1 minute)
setInterval(() => {
  bundleCache.cleanup()
  tokenCache.cleanup()
}, 60 * 1000)

const server = Bun.serve({
  port: process.env.PORT || 3000,

  async fetch(req) {
    const url = new URL(req.url)

    // Access log
    log('access', `${req.method} ${url.pathname}`)

    // CORS
    if (req.method === 'OPTIONS') {
      return new Response(null, {
        headers: {
          'Access-Control-Allow-Origin': '*',
          'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
          'Access-Control-Allow-Headers': 'Content-Type',
        },
      })
    }

    // Health check
    if (url.pathname === '/health') {
      return Response.json({
        status: 'ok',
        uptime: process.uptime(),
        timestamp: new Date().toISOString(),
      })
    }

    // POST /api/tokens - Generate tokens for a URL
    if (url.pathname === '/api/tokens' && req.method === 'POST') {
      try {
        const body = await req.json()
        const chapterUrl = body.url

        if (!chapterUrl) {
          return Response.json({ error: 'Chapter URL is required' }, { status: 400 })
        }

        // Check token cache (TTL: 25 seconds)
        const cacheKey = `tokens:${chapterUrl}`
        const cached = tokenCache.get(cacheKey)
        if (cached) {
          log('cache', `Tokens served from cache for ${chapterUrl}`)
          return Response.json({
            ...cached,
            cached: true,
            expiresIn: Math.floor((cached.expiresAt - Date.now()) / 1000),
          })
        }

        // Download bundle (with 10 minute cache)
        let bundleCode = bundleCache.get('bundle')
        if (!bundleCode) {
          log('info', 'Downloading bundle from ToonLivre...')

          const response = await fetch('https://toonlivre.net', {
            headers: {
              'User-Agent':
                'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
              Accept:
                'text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8',
              'Accept-Language': 'pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7',
              'Accept-Encoding': 'gzip, deflate, br',
              'Cache-Control': 'no-cache',
              Pragma: 'no-cache',
            },
          })
          const html = await response.text()

          // Extract bundle URL
          const match = html.match(/<script[^>]*src="([^"]*index-[^"]*\.js)"/)
          if (!match) {
            throw new Error('Bundle not found in HTML')
          }

          const bundleUrl = match[1].startsWith('http')
            ? match[1]
            : `https://toonlivre.net${match[1]}`

          log('info', `Downloading bundle: ${bundleUrl}`)
          const bundleResponse = await fetch(bundleUrl, {
            headers: {
              'User-Agent':
                'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
              Accept: '*/*',
              'Accept-Language': 'pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7',
              'Accept-Encoding': 'gzip, deflate, br',
              Referer: 'https://toonlivre.net/',
            },
          })
          bundleCode = await bundleResponse.text()

          // Cache for 10 minutes
          bundleCache.set('bundle', bundleCode, 10 * 60)
          log('cache', 'Bundle stored in cache (TTL: 10 min)')
        } else {
          log('cache', 'Bundle loaded from cache')
        }

        // Generate tokens
        const executor = new TokenExecutor(bundleCode)
        const session = executor.generateSession()
        const passphrase = executor.generatePassphrase()
        const headers = await executor.generateHeaders(chapterUrl)

        const result = {
          session,
          passphrase,
          headers,
          strategy: executor.detectStrategy(),
          expiresAt: Date.now() + 25 * 1000, // 25 segundos
        }

        // Cache for 25 seconds
        tokenCache.set(cacheKey, result, 25)
        log('cache', `Tokens generated and stored for ${chapterUrl}`)

        return Response.json(
          {
            ...result,
            cached: false,
            expiresIn: 25,
          },
          {
            headers: {
              'Access-Control-Allow-Origin': '*',
              'Content-Type': 'application/json',
            },
          }
        )
      } catch (error) {
        log('error', `Error generating tokens: ${(error as Error).message}`)
        return Response.json(
          {
            error: 'Error generating tokens',
            message: (error as Error).message,
          },
          { status: 500 }
        )
      }
    }

    // POST /api/decrypt - Decrypt data
    if (url.pathname === '/api/decrypt' && req.method === 'POST') {
      try {
        const body = await req.json()
        const { encrypted, passphrase } = body

        if (!encrypted) {
          return Response.json({ error: 'Encrypted data is required' }, { status: 400 })
        }

        // Get bundle from cache
        const bundleCode = bundleCache.get('bundle')
        if (!bundleCode) {
          return Response.json(
            { error: 'Bundle not found. Make a request to /api/tokens first.' },
            { status: 400 }
          )
        }

        const executor = new TokenExecutor(bundleCode)
        const decrypted = executor.decrypt(encrypted, passphrase)

        log('decrypt', 'Decryption successful')
        return Response.json(
          {
            decrypted,
          },
          {
            headers: {
              'Access-Control-Allow-Origin': '*',
              'Content-Type': 'application/json',
            },
          }
        )
      } catch (error) {
        log('error', `Error decrypting: ${(error as Error).message}`)
        return Response.json(
          {
            error: 'Error decrypting',
            message: (error as Error).message,
          },
          { status: 500 }
        )
      }
    }

    // GET /api/cache/stats - Cache statistics
    if (url.pathname === '/api/cache/stats') {
      return Response.json({
        bundle: bundleCache.has('bundle') ? 'cached' : 'empty',
        tokens: Array.from((tokenCache as any).store.keys()).length,
      })
    }

    // POST /api/cache/clear - Clear cache
    if (url.pathname === '/api/cache/clear' && req.method === 'POST') {
      bundleCache.clear()
      tokenCache.clear()
      log('info', 'Cache cleared manually')
      return Response.json({ message: 'Cache cleared successfully' })
    }

    return Response.json({ error: 'Endpoint not found' }, { status: 404 })
  },
})

console.log(`
==================================================================
  TOONLIVRE TOKEN SERVER
==================================================================

Server running at: http://localhost:${server.port}

Available endpoints:

  POST   /api/tokens         Generate tokens for chapter URL
  POST   /api/decrypt        Decrypt data
  GET    /api/cache/stats    Cache statistics
  POST   /api/cache/clear    Clear cache
  GET    /health             Health check

Cache configuration:

  Bundle:  10 minutes TTL
  Tokens:  25 seconds TTL

==================================================================
`)

export { server }
