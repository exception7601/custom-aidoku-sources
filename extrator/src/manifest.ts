import * as z from 'zod';

export const signatureRuleSchema = z
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

export const sessionCookieGeneratorSchema = z.object({
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

export const passphraseSchema = z.object({
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

export const manifestSchema = z.object({
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
  request: z.object({
    userAgent: z.string().min(1),
    acceptLanguage: z.string().min(1),
    signatureHeader: z.string().min(1),
    signatureRules: z.array(signatureRuleSchema).min(1),
    verifyHeader: z.string().min(1),
    includeCredentials: z.boolean(),
    sessionCookie: z.object({
      name: z.string().min(1),
      generator: sessionCookieGeneratorSchema,
      mirrorsInto: z.array(z.string().min(1)).min(1),
    }),
  }),
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
export type SessionCookieGenerator = z.infer<typeof sessionCookieGeneratorSchema>;
export type PassphraseStrategy = z.infer<typeof passphraseSchema>;
export type ExtractedManifest = z.infer<typeof manifestSchema>;

export function parseManifest(input: unknown): ExtractedManifest {
  return manifestSchema.parse(input);
}
