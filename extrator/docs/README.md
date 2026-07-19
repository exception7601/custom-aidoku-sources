# ToonLivre extractor

This project reads the current ToonLivre web bundle and generates a small manifest for the Aidoku source.
The goal is to keep unstable frontend details out of `sources/pt_BR.toonlivre`.

## What it does

- downloads the current bundle;
- parses the JavaScript into an AST;
- extracts request header rules, session token rules, and decrypt rules;
- writes a manifest that the source can use.

## Main files

- `src/cli.ts` has the `extract` and `validate` commands;
- `src/extract.ts` runs the full extraction flow;
- `src/manifest.ts` defines the manifest schema;
- `src/runtime.ts` rebuilds request and decrypt behavior from the manifest;
- `src/recognizers/` contains the bundle analysis logic.

## Inputs

The extractor accepts:

- a local bundle file;
- a direct bundle URL;
- the site base URL, which triggers HTML discovery.

## Outputs

The workflow writes:

- `manifest/manifest.json` as the latest manifest;
- `manifest/manifest_vYYYYMMDD-HHMM.json` as a dated snapshot;
- `sources/pt_BR.toonlivre/res/manifest.json` as the bundled fallback for the source.

## Tests

- `tests/unit/` covers parsing, recognizers, and runtime helpers;
- `tests/live/` covers live extraction and a real chapter request.

## Source integration

The source uses the manifest in this order:

- cached manifest;
- remote manifest;
- bundled fallback manifest.

This keeps the source working even if the remote manifest is unavailable.
