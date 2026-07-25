#!/usr/bin/env bun

/**
 * Test with real chapter from ToonLivre
 */

console.log(`
------------------------------------------------------------------
  TEST WITH REAL CHAPTER
------------------------------------------------------------------
`)

const SERVER_URL = 'http://localhost:3000'
const CHAPTER_URL = 'https://toonlivre.net/api/mangas/obra-dbbabf0f/chapters/cap-dd9e898d-522_5'

console.log(`\nChapter URL: ${CHAPTER_URL}\n`)

async function testRealChapter() {
  try {
    // 1. Generate tokens
    console.log('--------------------------------------------------')
    console.log('1. Generating tokens...')
    console.log('--------------------------------------------------')
    
    const tokenStart = Date.now()
    const tokenResponse = await fetch(`${SERVER_URL}/api/tokens`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: CHAPTER_URL }),
    })

    if (!tokenResponse.ok) {
      throw new Error(`Error generating tokens: ${tokenResponse.status}`)
    }

    const tokens = await tokenResponse.json()
    const tokenTime = Date.now() - tokenStart

    console.log(`Tokens generated in ${tokenTime}ms`)
    console.log(`   Session: ${tokens.session}`)
    console.log(`   Passphrase: ${tokens.passphrase}`)
    console.log(`   Strategy: ${tokens.strategy}`)
    console.log(`   Signature: ${tokens.headers['x-toon-signature'].substring(0, 50)}...`)
    console.log(`   Verify: ${tokens.headers['x-toon-verify']}`)
    console.log(`   Cached: ${tokens.cached}`)
    console.log(`   Expires in: ${tokens.expiresIn}s`)

    // 2. Request to ToonLivre
    console.log('\n--------------------------------------------------')
    console.log('2. Requesting chapter from ToonLivre...')
    console.log('--------------------------------------------------')

    const chapterStart = Date.now()
    const chapterResponse = await fetch(CHAPTER_URL, {
      headers: {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
        'Accept': 'application/json, text/plain, */*',
        'Accept-Language': 'pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7',
        'Referer': 'https://toonlivre.net/',
        'x-toon-signature': tokens.headers['x-toon-signature'],
        'x-toon-verify': tokens.headers['x-toon-verify'],
      },
    })

    const chapterTime = Date.now() - chapterStart

    console.log(`Status: ${chapterResponse.status} ${chapterResponse.statusText}`)
    console.log(`Time: ${chapterTime}ms`)
    
    if (!chapterResponse.ok) {
      console.log('Request failed')
      console.log(`   Possible reasons:`)
      console.log(`   • Chapter URL does not exist`)
      console.log(`   • Chapter removed`)
      console.log(`   • ToonLivre API format changed`)
      console.log(`   • Tokens invalid`)
      return
    }

    const data = await chapterResponse.json()
    console.log('Request successful!')
    
    // 3. Analyze response
    console.log('\n--------------------------------------------------')
    console.log('3. Analyzing response...')
    console.log('--------------------------------------------------')

    console.log(`   Data type: ${typeof data}`)
    console.log(`   Is array: ${Array.isArray(data)}`)
    
    if (typeof data === 'object') {
      const keys = Object.keys(data)
      console.log(`   Fields: ${keys.join(', ')}`)
      
      // LOG DATA RECEIVED
      console.log('DATA RECEIVED (first 500 characters):')
      console.log(JSON.stringify(data, null, 2).substring(0, 500))
      
      // Find encrypted field dynamically
      const encryptedKey = keys[0];
      const encryptedValue = data[encryptedKey];
      
      console.log(`Data appears to be encrypted in field: ${encryptedKey}`)
      
      // 4. Try to decrypt
      console.log('--------------------------------------------------')
      console.log('4. Decrypting...')
      console.log('--------------------------------------------------')

      const decryptResponse = await fetch(`${SERVER_URL}/api/decrypt`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          encrypted: encryptedValue,
          passphrase: tokens.passphrase,
        }),
      })

      if (!decryptResponse.ok) {
        console.log(`Error decrypting: ${decryptResponse.status}`)
        return
      }

      const decrypted = await decryptResponse.json()
      console.log('Decryption successful!')
      
      // Display decrypted data
      console.log(JSON.stringify(decrypted, null, 2).substring(0, 500))
    } else {
      console.log(`   Data does not appear to be encrypted`)
    }

    console.log('--------------------------------------------------')
    console.log('TEST COMPLETE')
    console.log('--------------------------------------------------')
    console.log(`
  Tokens generated (${tokenTime}ms)
  Request to ToonLivre (${chapterTime}ms)
  Data received
  Decryption worked
  
  Integration with ToonLivre is working!
`)
    console.log('--------------------------------------------------\n')

  } catch (error) {
    console.log('\nError in test:', (error as Error).message)
    console.log('\nPossible causes:')
    console.log('  • Server not running (run: bun run dev)')
    console.log('  • Chapter URL changed')
    console.log('  • ToonLivre blocked access')
    console.log('  • Tokens invalid')
    process.exit(1)
  }
}

testRealChapter()
