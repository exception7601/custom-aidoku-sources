# ToonLivre extractor

This project reads the current ToonLivre web bundle and generates a small manifest for the Aidoku source.
The goal is to keep unstable frontend details out of `sources/pt_BR.toonlivre`.

## What it does

- fetches the current bundle when extraction needs it;
- parses the JavaScript into an AST;
- extracts request header rules, session token rules, and decrypt rules;
- validates the extracted runtime against a live chapter endpoint;
- writes a manifest that the source can use;
- saves bundle snapshots only through `download-bundle` or `scripts/refresh-manifest.sh`.

## Main files

- `src/cli.ts` exposes the `extract`, `validate`, `probe`, `compat`, and `download-bundle` commands;
- `src/extract.ts` runs the full extraction flow;
- `src/manifest.ts` defines the manifest schema;
- `src/runtime.ts` rebuilds request and decrypt behavior from the manifest;
- `src/validate.ts` checks whether a manifest still works against a live chapter API;
- `src/probe.ts` compares the saved manifest bundle against the live site;
- `src/recognizers/` contains the bundle analysis logic.

## Basic commands

Install dependencies.

```sh
env -C extrator npm install
```

Build the compiled CLI when `dist/cli.js` is missing or stale.

```sh
env -C extrator npm run build
```

Run the unit tests.

```sh
env -C extrator npm test
```

Run the bundle baseline compatibility checks.
This reuses the existing compiled CLI and does not build automatically.
If `dist/cli.js` is missing, run `env -C extrator npm run build` manually first.

```sh
env -C extrator npm run test:compat
```

Run the live extractor test suite.

```sh
env -C extrator npm run test:live
```

Run lint and typecheck.

```sh
env -C extrator npm run lint
env -C extrator npm run typecheck
```

Run dead code checks for the extractor TypeScript code.
This uses `knip` from `mise` and treats `tests/**/*.ts` as entry points.
Fixture files under `tests/fixtures/` are loaded by helper name and should still be audited separately.

```sh
env -C extrator npm run knip
```

Validate the current extractor manifest against the live chapter canary.

```sh
env -C extrator node dist/cli.js validate --manifest manifest/manifest.json
```

Probe whether the saved manifest bundle still matches the live site.

```sh
env -C extrator node dist/cli.js probe --manifest manifest/manifest.json
```

Extract a fresh manifest from the live site without saving a bundle snapshot.

```sh
env -C extrator npm run extract -- --entry-url https://toonlivre.net/
```

Download the current live bundle snapshot.

```sh
env -C extrator node dist/cli.js download-bundle --entry-url https://toonlivre.net/ --out-dir bundles
```

Compare one saved bundle file against its baseline manifest.

```sh
env -C extrator node dist/cli.js compat --bundle-file bundles/bundle_v1784634648_index-CMe0Aw9p_js/index-CMe0Aw9p.js
```

## Inputs

The extractor accepts:

- a local bundle file;
- a direct bundle URL;
- the site base URL, which triggers HTML discovery.

## Outputs

The workflow writes:

- `extrator/manifest/manifest.json` as the latest manifest;
- `extrator/manifest/baselines/*.json` as per-bundle compatibility baselines;
- `extrator/bundles/bundle_v*/` as saved bundle snapshots, written only by `download-bundle` or `scripts/refresh-manifest.sh`;
- `sources/pt_BR.toonlivre/res/manifest.json` as the bundled fallback for the source.

## Tests

- `tests/unit/` covers parsing, recognizers, runtime helpers, and bundle baseline compatibility;
- `tests/live/` covers live extraction and a real chapter request without persisting bundle snapshots.

## Source integration

The source uses the manifest in this order:

- cached manifest;
- remote manifest;
- bundled fallback manifest.

This keeps the source working even if the remote manifest is unavailable.

## Recommended update flow

When ToonLivre changes, use this sequence.

- run `probe` to confirm whether the live bundle changed;
- run `scripts/refresh-manifest.sh` when you want to save the live bundle snapshot and refresh the baseline manifest from that saved file;
- run `compat` to confirm the saved bundle snapshots still match their baseline manifests;
- run `validate` against `manifest/manifest.json`;
- sync the manifest into `sources/pt_BR.toonlivre/res/manifest.json` when you are ready to update the source fallback;
- run `cargo test` in `sources/pt_BR.toonlivre`;
- package and verify the source if the tests pass.
