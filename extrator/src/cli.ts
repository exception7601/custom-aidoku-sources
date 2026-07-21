#!/usr/bin/env node

import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

import { Command } from 'commander';

import { DEFAULT_CANARY_CHAPTER_URL, DEFAULT_SITE_URL, DEFAULT_SOURCE_ID } from './constants.js';
import {
  buildCompatibilityFailureMessage,
  checkArchivedManifestCompatibility,
  checkBundleCompatibility,
} from './compatibility.js';
import { downloadBundle } from './download-bundle.js';
import { extractManifest } from './extract.js';
import { parseManifest } from './manifest.js';
import { probeManifestBundle } from './probe.js';
import { validateManifestAgainstChapter } from './validate.js';

export async function runCli(argv: string[]): Promise<void> {
  const program = new Command();

  program
    .name('toonlivre-extrator')
    .description('Extracts a declarative ToonLivre request/decrypt manifest from app bundles.')
    .version('0.1.0');

  program
    .command('extract')
    .description('Extract a manifest from ToonLivre HTML, bundle URLs, or local bundle files.')
    .option('--source-id <sourceId>', 'Manifest source identifier', DEFAULT_SOURCE_ID)
    .option('--site-url <siteUrl>', 'Base site URL', DEFAULT_SITE_URL)
    .option('--entry-url <entryUrl>', 'HTML entry URL to inspect for script tags')
    .option(
      '--bundle-url <bundleUrls...>',
      'Explicit bundle URLs to inspect. Pass the base site URL to trigger discovery.'
    )
    .option('--bundle-file <bundleFiles...>', 'Local bundle files to inspect')
    .option('--out <path>', 'Write the manifest JSON to a file')
    .option('--pretty', 'Pretty-print JSON output', true)
    .option('--validate', 'Validate the extracted manifest against a live chapter endpoint', false)
    .option(
      '--canary-chapter-url <chapterUrl>',
      'Live chapter API URL used by `--validate`',
      DEFAULT_CANARY_CHAPTER_URL
    )
    .action(async (options) => {
      const manifest = await extractManifest({
        sourceId: options.sourceId,
        siteUrl: options.siteUrl,
        entryUrl: options.entryUrl,
        bundleUrls: options.bundleUrl,
        bundleFiles: options.bundleFile,
      });

      if (options.validate) {
        const validation = await validateManifestAgainstChapter(manifest, options.canaryChapterUrl);
        console.error(
          `[validate] status=${validation.status} pageCount=${validation.pageCount} ` +
            `dataKey=${validation.dataKey ?? 'none'}`
        );
      }

      const serialized = JSON.stringify(manifest, null, options.pretty ? 2 : undefined);
      if (options.out) {
        const outputPath = resolve(options.out);
        await writeFile(outputPath, `${serialized}\n`, 'utf8');
        console.error(`[write] manifest saved to ${outputPath}`);
      }

      process.stdout.write(`${serialized}\n`);
    });

  program
    .command('probe')
    .description('Quickly detect whether the live ToonLivre bundle changed since a saved manifest.')
    .requiredOption('--manifest <path>', 'Manifest JSON file to compare against the live site')
    .option('--site-url <siteUrl>', 'Base site URL to inspect', DEFAULT_SITE_URL)
    .option('--entry-url <entryUrl>', 'HTML entry URL to inspect for script tags')
    .action(async (options) => {
      const input = await readFile(resolve(options.manifest), 'utf8');
      const manifest = parseManifest(JSON.parse(input));
      const probe = await probeManifestBundle({
        manifest,
        siteUrl: options.siteUrl,
        entryUrl: options.entryUrl,
      });

      process.stdout.write(`${JSON.stringify(probe, null, 2)}\n`);
    });

  program
    .command('validate')
    .description('Validate a saved manifest against a live ToonLivre chapter endpoint.')
    .requiredOption('--manifest <path>', 'Manifest JSON file to load')
    .option(
      '--canary-chapter-url <chapterUrl>',
      'Live chapter API URL to request',
      DEFAULT_CANARY_CHAPTER_URL
    )
    .action(async (options) => {
      const input = await readFile(resolve(options.manifest), 'utf8');
      const manifest = parseManifest(JSON.parse(input));
      const validation = await validateManifestAgainstChapter(manifest, options.canaryChapterUrl);

      process.stdout.write(`${JSON.stringify(validation, null, 2)}\n`);
    });

  program
    .command('compat')
    .description('Compare saved bundle snapshots against baseline manifests.')
    .option('--bundle-file <path>', 'Check one bundle file instead of the whole baseline set')
    .option('--manifest <path>', 'Expected manifest for `--bundle-file`')
    .option('--manifest-dir <path>', 'Directory containing per-bundle baseline manifests')
    .option('--bundles-dir <path>', 'Directory containing saved bundle snapshots')
    .option('--site-url <siteUrl>', 'Base site URL', DEFAULT_SITE_URL)
    .option('--source-id <sourceId>', 'Manifest source identifier', DEFAULT_SOURCE_ID)
    .option('--entry-url <entryUrl>', 'Entry URL to stamp on extracted manifests when needed')
    .action(async (options) => {
      if (options.bundleFile) {
        const result = await checkBundleCompatibility({
          bundleFile: options.bundleFile,
          manifestPath: options.manifest,
          manifestDir: options.manifestDir,
          sourceId: options.sourceId,
          siteUrl: options.siteUrl,
          entryUrl: options.entryUrl,
        });

        if (!result.ok) {
          throw new Error(
            buildCompatibilityFailureMessage({
              bundleFile: result.bundleFile,
              manifestPath: result.expectedManifestPath,
              actual: result.actual,
              expected: result.expected,
            })
          );
        }

        process.stdout.write(
          `${JSON.stringify(
            {
              ok: true,
              bundleFile: result.bundleFile,
              expectedManifestPath: result.expectedManifestPath,
              bundleHash: result.bundleHash,
            },
            null,
            2
          )}\n`
        );
        return;
      }

      const results = await checkArchivedManifestCompatibility({
        manifestDir: options.manifestDir,
        bundlesDir: options.bundlesDir,
        sourceId: options.sourceId,
        siteUrl: options.siteUrl,
      });
      const failed = results.filter((result) => !result.ok);

      if (failed.length > 0) {
        throw new Error(
          failed
            .map((result) =>
              buildCompatibilityFailureMessage({
                bundleFile: result.bundleFile,
                manifestPath: result.manifestPath,
                actual: result.actual,
                expected: result.expected,
              })
            )
            .join('\n\n')
        );
      }

      process.stdout.write(
        `${JSON.stringify(
          {
            ok: true,
            checked: results.length,
            manifests: results.map((result) => ({
              manifestPath: result.manifestPath,
              bundleFile: result.bundleFile,
              bundleHash: result.bundleHash,
            })),
          },
          null,
          2
        )}\n`
      );
    });

  program
    .command('download-bundle')
    .description('Download the current ToonLivre app bundle and snapshot bundle diagnostics.')
    .option('--site-url <siteUrl>', 'Base site URL', DEFAULT_SITE_URL)
    .option('--entry-url <entryUrl>', 'HTML entry URL to inspect for script tags')
    .option(
      '--bundle-url <bundleUrls...>',
      'Explicit bundle URLs to download. Pass the base site URL to trigger discovery.'
    )
    .option('--out-dir <path>', 'Directory where bundle snapshots are stored', 'bundles')
    .action(async (options) => {
      const result = await downloadBundle({
        siteUrl: options.siteUrl,
        entryUrl: options.entryUrl,
        bundleUrls: options.bundleUrl,
        outputDir: options.outDir,
      });

      process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    });

  await program.parseAsync(argv);
}

if (import.meta.url === `file://${process.argv[1]}`) {
  runCli(process.argv).catch((error: unknown) => {
    const message = error instanceof Error ? (error.stack ?? error.message) : String(error);
    console.error(message);
    process.exitCode = 1;
  });
}
