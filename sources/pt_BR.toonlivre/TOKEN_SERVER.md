# Token Server Configuration

This document explains how to configure and use an external token server for the ToonLivre source.

## Configuration File

Location: `res/token-server.json`

```json
{
  "schemaVersion": 1,
  "enabled": false,
  "host": "https://toons.4nd.xyz",
  "endpoints": {
    "tokens": "/api/tokens",
    "decrypt": "/api/decrypt",
    "health": "/health"
  },
  "timeout": {
    "connect": 10,
    "request": 30
  },
  "cache": {
    "enabled": true,
    "ttl": 20
  },
  "fallback": {
    "useLocalManifest": true,
    "retryCount": 2
  }
}
```

## Configuration Options

### `schemaVersion`
- Type: `integer`
- Required: Yes
- Description: Schema version for compatibility checks
- Default: `1`

### `enabled`
- Type: `boolean`
- Required: Yes
- Description: Enable or disable the external token server
- Default: `false`
- **Note**: Set to `true` to use the external server

### `host`
- Type: `string`
- Required: Yes
- Description: Base URL of the token server
- Example: `"https://toons.4nd.xyz"`
- **Note**: Do not include trailing slash

### `endpoints`
Object containing API endpoint paths:

#### `endpoints.tokens`
- Type: `string`
- Required: Yes
- Description: Path for token generation endpoint
- Default: `"/api/tokens"`
- Usage: `POST {host}{endpoints.tokens}`

#### `endpoints.decrypt`
- Type: `string`
- Required: Yes
- Description: Path for decryption endpoint
- Default: `"/api/decrypt"`
- Usage: `POST {host}{endpoints.decrypt}`

#### `endpoints.health`
- Type: `string`
- Required: Yes
- Description: Path for health check endpoint
- Default: `"/health"`
- Usage: `GET {host}{endpoints.health}`

### `timeout`
Object containing timeout configurations:

#### `timeout.connect`
- Type: `integer`
- Required: Yes
- Description: Connection timeout in seconds
- Default: `10`
- Range: `1-60`

#### `timeout.request`
- Type: `integer`
- Required: Yes
- Description: Request timeout in seconds
- Default: `30`
- Range: `5-120`

### `cache`
Object containing cache configurations:

#### `cache.enabled`
- Type: `boolean`
- Required: Yes
- Description: Enable local caching of server responses
- Default: `true`

#### `cache.ttl`
- Type: `integer`
- Required: Yes
- Description: Cache time-to-live in seconds
- Default: `20`
- Range: `5-300`

### `fallback`
Object containing fallback behavior:

#### `fallback.useLocalManifest`
- Type: `boolean`
- Required: Yes
- Description: Fall back to local manifest if server fails
- Default: `true`

#### `fallback.retryCount`
- Type: `integer`
- Required: Yes
- Description: Number of retry attempts before fallback
- Default: `2`
- Range: `0-5`

## How It Works

### When Enabled (`enabled: true`)

1. **Token Generation**
   - Request: `POST https://toons.4nd.xyz/api/tokens`
   - Body: `{"url": "https://toonlivre.net/api/mangas/{id}/chapters/{chapterId}"}`
   - Response:
     ```json
     {
       "session": "abc123...",
       "passphrase": "Vortex-Blade-Nexus4b97f079c",
       "headers": {
         "x-toon-signature": "eyJhbGc...",
         "x-toon-verify": "abc123..."
       },
       "strategy": "seed-jwt",
       "cached": false,
       "expiresIn": 25
     }
     ```

2. **Decryption** (if needed)
   - Request: `POST https://toons.4nd.xyz/api/decrypt`
   - Body: `{"encrypted": "U2FsdGVk...", "passphrase": "Vortex-..."}`
   - Response:
     ```json
     {
       "decrypted": "{\"id\":\"cap-...\",\"pages\":[...]}"
     }
     ```

3. **Health Check**
   - Request: `GET https://toons.4nd.xyz/health`
   - Response:
     ```json
     {
       "status": "ok",
       "uptime": 3600,
       "timestamp": "2026-07-25T17:39:11.534Z"
     }
     ```

### When Disabled (`enabled: false`)

- Uses local manifest (`res/manifest.json`)
- Generates tokens and passphrases locally
- Falls back to bundled configuration

## Fallback Behavior

If the server is enabled but fails:

1. **First Attempt**: Try server with configured timeout
2. **Retry**: Retry up to `fallback.retryCount` times
3. **Fallback**: If `fallback.useLocalManifest` is `true`, use local manifest
4. **Error**: If fallback disabled, return error to user

## Error Handling

### Server Errors
- Connection timeout → Fallback to local
- HTTP 500/502/503 → Retry then fallback
- HTTP 429 (Rate Limit) → Wait and retry
- Invalid response → Fallback to local

### Network Errors
- DNS resolution failed → Fallback to local
- Network unreachable → Fallback to local
- SSL/TLS errors → Fallback to local

## Performance Considerations

### Cache Strategy
- **Cache TTL**: 20 seconds (default)
- **Cache Key**: Chapter URL
- **Cache Storage**: In-memory (per session)

### Request Flow
```
User Request
    ↓
Check Cache (TTL: 20s)
    ↓ (miss)
Request Token Server
    ↓ (success)
Cache Response
    ↓
Use Tokens
```

### Bandwidth Usage
- Token request: ~500 bytes
- Token response: ~1 KB
- Cached locally for 20s
- Estimate: ~3 KB per chapter (with retries)

## Deployment

### Self-Hosted Server

1. Deploy the token server (from `token-server/` directory)
2. Configure DNS/domain
3. Update `token-server.json`:
   ```json
   {
     "enabled": true,
     "host": "https://your-domain.com"
   }
   ```
4. Test with health check

### Using Default Server

1. Keep default configuration:
   ```json
   {
     "enabled": true,
     "host": "https://toons.4nd.xyz"
   }
   ```
2. No additional setup required

## Testing

### Test Health Check
```bash
curl https://toons.4nd.xyz/health
```

### Test Token Generation
```bash
curl -X POST https://toons.4nd.xyz/api/tokens \
  -H "Content-Type: application/json" \
  -d '{"url": "https://toonlivre.net/api/mangas/test/chapters/test"}'
```

### Test Decryption
```bash
curl -X POST https://toons.4nd.xyz/api/decrypt \
  -H "Content-Type: application/json" \
  -d '{"encrypted": "U2FsdGVk...", "passphrase": "Vortex-..."}'
```

## Troubleshooting

### Server Not Responding
1. Check `enabled` is `true`
2. Verify `host` URL is correct
3. Test health endpoint manually
4. Check network connectivity
5. Review timeout settings

### Slow Performance
1. Reduce `timeout.request` for faster fallback
2. Increase `cache.ttl` for fewer requests
3. Check server response times
4. Consider self-hosting closer to users

### Fallback Not Working
1. Verify `fallback.useLocalManifest` is `true`
2. Check `res/manifest.json` exists and is valid
3. Review logs for error messages
4. Test local manifest independently

## Security Considerations

### HTTPS Required
- Always use `https://` for production
- Never use `http://` (insecure)

### Rate Limiting
- Server implements rate limits
- Respect HTTP 429 responses
- Implement exponential backoff

### Data Privacy
- Chapter URLs are sent to server
- No personal information transmitted
- Server logs may contain URLs

## Migration Guide

### From Local to Server

1. Keep `enabled: false` initially
2. Test server independently
3. Enable server: `enabled: true`
4. Monitor for issues
5. Adjust timeouts if needed

### From Server to Local

1. Set `enabled: false`
2. Verify local manifest is current
3. Test chapter downloads
4. Update manifest if needed

## Future Enhancements

Planned features:
- [ ] Authentication/API keys
- [ ] Regional server routing
- [ ] Automatic failover
- [ ] Server-side caching
- [ ] Metrics and monitoring

## Support

For issues or questions:
- Repository: https://github.com/exception7601/custom-aidoku-sources
- Token Server: `token-server/` directory
- Documentation: `token-server/README.md`
