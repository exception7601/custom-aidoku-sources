import * as z from 'zod';

const signatureRuleSchema = z
  .object({
    value: z.string().min(1),
    default: z.boolean().optional(),
    when: z
      .object({
        urlContains: z.string().min(1),
      })
      .optional(),
  })
  .refine((rule) => rule.default === true || rule.when !== undefined, {
    message: 'A signature rule must define `default` or `when`.',
  });

const timeSha256Base64SignatureStrategySchema = z.object({
  kind: z.literal('time-sha256-base64'),
  timestampDivisor: z.number().int().positive(),
  salt: z.string().min(1),
  routeSelector: z.object({
    whenUrlContains: z.string().min(1),
    whenMatched: z.string().min(1),
    otherwise: z.string().min(1),
  }),
});

const seedJwtSignatureStrategySchema = z.object({
  kind: z.literal('seed-jwt'),
  metaName: z.string().min(1),
  endpointPath: z.string().min(1),
  tokenField: z.string().min(1),
});

const dynamicSignatureStrategySchema = z.discriminatedUnion('kind', [
  timeSha256Base64SignatureStrategySchema,
  seedJwtSignatureStrategySchema,
]);

const sessionCookieGeneratorSchema = z.object({
  kind: z.literal('random-base36-concat'),
  segments: z
    .array(
      z.object({
        radix: z.number().int().min(2).max(36),
        start: z.number().int().min(0),
        end: z.number().int().min(1),
      })
    )
    .min(1),
});

const utcMd5DerivedPassphraseSchema = z.object({
  kind: z.literal('utc-md5-derived'),
  dateFormat: z.literal('YYYY-MM-DD'),
  prefix: z.string(),
  salt: z.string(),
  suffix: z.string(),
  digestEncoding: z.literal('hex'),
  digestSlice: z.object({
    start: z.number().int().min(0),
    end: z.number().int().positive(),
  }),
});

const utcSha256DerivedPassphraseSchema = z.object({
  kind: z.literal('utc-sha256-derived'),
  dateFormat: z.literal('YYYY-MM-DD'),
  prefix: z.string(),
  salt: z.string(),
  suffix: z.string(),
  digestEncoding: z.literal('hex'),
  digestSlice: z.object({
    start: z.number().int().min(0),
    end: z.number().int().positive(),
  }),
});

const passphraseSchema = z.discriminatedUnion('kind', [
  utcMd5DerivedPassphraseSchema,
  utcSha256DerivedPassphraseSchema,
]);

const manifestSchema = z.object({
  schemaVersion: z.literal(1),
  sourceId: z.string().min(1),
  siteUrl: z.string().url(),
  entryUrl: z.string().url().optional(),
  extractedAt: z.string().datetime(),
  bundle: z.object({
    url: z.string().url().optional(),
    hash: z.string().min(1),
    discoveredFrom: z.enum(['cli', 'html', 'file']),
  }),
  request: z
    .object({
      userAgent: z.string().min(1),
      acceptLanguage: z.string().min(1),
      signatureHeader: z.string().min(1),
      signatureRules: z.array(signatureRuleSchema),
      signatureStrategy: dynamicSignatureStrategySchema.optional(),
      verifyHeader: z.string().min(1).optional(),
      includeCredentials: z.boolean(),
      sessionCookie: z.object({
        name: z.string().min(1),
        generator: sessionCookieGeneratorSchema,
        mirrorsInto: z.array(z.string().min(1)).default([]),
      }),
    })
    .refine(
      (request) => request.signatureRules.length > 0 || request.signatureStrategy !== undefined,
      {
        message: 'Request config must define `signatureRules` or `signatureStrategy`.',
        path: ['signatureRules'],
      }
    ),
  decrypt: z.object({
    dataKeyHeader: z.string().min(1),
    payloadSelector: z.literal('header-named-or-first-string'),
    algorithm: z.literal('cryptojs-rabbit'),
    passphrase: passphraseSchema,
  }),
  diagnostics: z
    .object({
      scriptUrls: z.array(z.string().url()).default([]),
      snippets: z.object({
        request: z.string().optional(),
        session: z.string().optional(),
        decrypt: z.string().optional(),
      }),
    })
    .optional(),
});

export type SignatureRule = z.infer<typeof signatureRuleSchema>;
export type DynamicSignatureStrategy = z.infer<typeof dynamicSignatureStrategySchema>;
export type SessionCookieGenerator = z.infer<typeof sessionCookieGeneratorSchema>;
export type PassphraseStrategy = z.infer<typeof passphraseSchema>;
export type ExtractedManifest = z.infer<typeof manifestSchema>;

export function parseManifest(input: unknown): ExtractedManifest {
  return manifestSchema.parse(input);
}
