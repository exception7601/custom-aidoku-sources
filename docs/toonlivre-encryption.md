# ToonLivre Encryption Strategy

## Overview

This document describes the encryption strategy used by ToonLivre website for their API communication.

## Current Status (July 2026)

**No encryption** - ToonLivre removed all encryption on July 21, 2026.

- Direct API access works without any encryption
- No tokens required
- Only `x-toon-route` header remains
- API responses are plain JSON

## Historical Encryption (Until July 21, 2026)

### Rabbit Cipher Encryption

ToonLivre used **Rabbit stream cipher** with a dynamic passphrase for encrypting API requests and responses.

#### 1. Passphrase Generation

```javascript
function nv() {
  const now = new Date();
  const utc = Date.UTC(
    now.getUTCFullYear(),
    now.getUTCMonth(),
    now.getUTCDate(),
    now.getUTCHours()
  );
  return MD5(utc.toString()).toString();
}
```

The passphrase was an MD5 hash of the current UTC time (truncated to the hour). This meant:
- Passphrase changed every hour
- Both client and server could independently generate the same passphrase
- No key exchange needed

#### 2. Request Encryption

```javascript
// Encrypt request data
const passphrase = nv(); // Generate current passphrase
const encrypted = CryptoJS.Rabbit.encrypt(
  JSON.stringify(requestData),
  passphrase
).toString();
```

#### 3. Response Decryption

```javascript
// Decrypt API response
const passphrase = nv();
const decrypted = CryptoJS.Rabbit.decrypt(
  encryptedResponse,
  passphrase
).toString(CryptoJS.enc.Utf8);
const data = JSON.parse(decrypted);
```

#### 4. HTTP Headers

```javascript
{
  "x-toon-signature": authToken,      // Authentication token
  "x-toon-verify": decoyToken,        // Decoy token
  "x-toon-datakey": encryptedData,    // Encrypted request data
  "x-toon-route": apiRoute            // API route
}
```

#### 5. Tokens

- **v9_auth_k8**: Primary authentication token
- **v9_decoy_k8**: Decoy token (anti-scraping measure)

Both tokens were stored in localStorage and rotated periodically.

## Why Encryption Was Removed

ToonLivre likely removed encryption because:

1. **Maintenance overhead** - Hourly passphrase rotation and token management
2. **Limited effectiveness** - Encryption logic was in client-side JavaScript (easily reverse-engineered)
3. **Performance** - Encryption/decryption added latency
4. **CDN caching** - Encrypted responses couldn't be cached effectively

## Proxy Implementation

### Current Strategy (No Encryption)

```typescript
// Direct API call
const response = await fetch('https://toonlivre.net/api/mangas/releases', {
  headers: {
    'x-toon-route': 'mangas.releases',
    'Cookie': `toon_i=${sessionId}`
  }
});
const data = await response.json();
```

### Fallback Strategy (If Encryption Returns)

The proxy implements automatic fallback detection:

```typescript
// Try direct access first
try {
  const response = await directRequest();
  if (response.status === 401 || response.status === 403) {
    // Switch to encryption mode
    USE_ENCRYPTION = true;
  }
  return response;
} catch (error) {
  // Activate encryption fallback
  return await encryptedRequest();
}
```

### Detection Triggers

The proxy switches to encryption mode when:

1. HTTP status 401/403
2. Error messages about missing tokens or signatures
3. Response body contains encrypted data patterns
4. API returns token-related errors

## Token Server (Legacy)

The `token-server` implemented the encryption strategy:

```
token-server/
├── src/
│   ├── crypto.ts          # Rabbit + MD5 implementation
│   ├── token-manager.ts   # Token management
│   └── index.ts           # API endpoints
```

### Endpoints

- `POST /encrypt` - Encrypt data with current passphrase
- `POST /decrypt` - Decrypt data with current passphrase
- `GET /passphrase` - Get current passphrase
- `GET /tokens` - Get authentication tokens

## Bundle Analysis

JavaScript bundles extracted from ToonLivre contain the encryption logic:

```
extrator/bundles/
├── bundle_v1784613078_index-BHPG5Mhr_js  # Last bundle with encryption (Jul 21)
├── bundle_v1784960029_index-Cekp7BsG_js  # First bundle without encryption (Jul 25)
└── CHANGELOG.md                           # Bundle history
```

### Key Changes in Bundles

**With Encryption (≤ Jul 21, 2026)**:
- Contains `CryptoJS.Rabbit` implementation
- `nv()` function for passphrase generation
- Token management logic
- Signature verification

**Without Encryption (≥ Jul 21, 2026)**:
- No `CryptoJS.Rabbit`
- No `nv()` function
- No token generation
- Simple session cookies only

## Monitoring Strategy

The proxy checks every 5 minutes:

```typescript
setInterval(async () => {
  const needsEncryption = await checkEncryptionRequired();
  if (needsEncryption !== USE_ENCRYPTION) {
    USE_ENCRYPTION = needsEncryption;
    console.log(`[api] Encryption mode ${needsEncryption ? 'activated' : 'deactivated'}`);
  }
}, 5 * 60 * 1000);
```

## Security Implications

### Why Client-Side Encryption Doesn't Work

1. **Visible Logic**: All encryption code is in JavaScript bundles
2. **Static Passphrase**: MD5(UTC hour) can be replicated by anyone
3. **No Key Exchange**: No server verification of client identity
4. **Token Theft**: Tokens in localStorage are accessible

### Actual Protection

The encryption primarily served to:
- Deter casual scrapers
- Make automated tools slightly harder to build
- Add obfuscation layer

It did **not** provide:
- Real security
- Protection against determined reverse-engineering
- Authentication or authorization

## Implementation Notes

### For Source Developers

When implementing sources that access ToonLivre:

1. **Use the proxy** - It handles encryption detection automatically
2. **Don't implement encryption** - Proxy handles fallback if needed
3. **Monitor logs** - Check if encryption mode activates
4. **Test regularly** - API strategy may change

### Current Recommendations

```typescript
// Simple and future-proof
const response = await fetch('http://proxy:4000/api/releases?page=1');
const data = await response.json();
```

The proxy will:
- Try direct access first (fast)
- Detect if encryption is needed
- Switch to token-server automatically
- Handle all complexity transparently

## References

- Bundle history: `extrator/bundles/CHANGELOG.md`
- Token server: `../token-server/` (deprecated, kept for fallback)
- Proxy implementation: `../toons-proxy/src/toonlivre-api.ts`
- Detection logic: `toons-proxy/src/toonlivre-api.ts:checkIfEncryptionNeeded()`

## Timeline

- **Jul 19, 2026**: Last confirmed encryption active
- **Jul 21, 2026 05:51 UTC**: Bundle `BHPG5Mhr` (with encryption)
- **Jul 21, 2026 08:29 UTC**: Bundle `CMe0Aw9p` (no encryption)
- **Jul 25, 2026**: Confirmed no encryption required
- **Jul 27, 2026**: Proxy v2.0.0 with automatic fallback released
