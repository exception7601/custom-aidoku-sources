## Sources

- `sources/pt_BR.montetaiscanlator`

## Folder overview

- `sources/` contains source projects.
- `public/` contains generated source list artifacts.
- `public/` is generated locally and published by CI to the `gh-pages` branch.
- `public/index.json` and `public/index.min.json` are the list entrypoints.
- `public/sources/` contains packaged `.aix` outputs.
- `public/icons/` contains icon assets copied during list build.

## Creating a new source

Create the source directory first.

```sh
mkdir sources/pt_BR.<Example>
```

Then initialize the source from the repo root.
Do not use `env -C` for this step.
Pass the destination path as the last argument.

```sh
aidoku init --name "<Example>" --url "https://<Example>.com" --content-rating safe --languages pt sources/pt_BR.<Example>
```

Replace `<Example>` and the URL with the real source values.

## Commands for sources

Format the source.

```sh
env -C sources/pt_BR.montetaiscanlator cargo fmt
```

## `extrator/`

For extractor usage, commands, and workflow notes, see `extrator/docs/README.md`.


Package the source.

```sh
env -C sources/pt_BR.montetaiscanlator aidoku package
```

Update dependencies for one source.
This updates that source's own `Cargo.lock`.

```sh
env -C sources/pt_BR.montetaiscanlator cargo update
```

Run source tests.
This command runs both offline parser checks and live site integration checks.

```sh
env -C sources/pt_BR.montetaiscanlator cargo test
```

Lint the source with Clippy.

```sh
env -C sources/pt_BR.montetaiscanlator cargo clippy
```

Verify the package.

```sh
aidoku verify sources/pt_BR.montetaiscanlator/package.aix
```

Build the public source list using packaged sources.

```sh
aidoku build sources/*/package.aix --name "Aidoku Custom Sources"
```

Serve the generated `public/` list.

```sh
aidoku serve public
```

Open the log server for device debugging.

```sh
aidoku logcat
```

## Release flow

Before publishing, always execute this sequence.

1. Bump `info.version` in `sources/pt_BR.montetaiscanlator/res/source.json`.
2. Run `env -C sources/pt_BR.montetaiscanlator cargo fmt`.
3. Run `env -C sources/pt_BR.montetaiscanlator cargo test`.
4. Run `env -C sources/pt_BR.montetaiscanlator aidoku package`.
5. Run `aidoku verify sources/pt_BR.montetaiscanlator/package.aix`.
6. Run `aidoku build sources/*/package.aix --name "Aidoku Custom Sources"`.

## Notes

- `cargo test` runs both offline parser checks and live site integration checks.
- Each source has its own `Cargo.lock`.
- When `aidoku-rs` or another dependency needs to change for one source, run `cargo update` inside that source directory and commit the updated `Cargo.lock`.
- After a dependency update, re-run `aidoku package` and `aidoku verify` for that source.
- Release CI caches `sources/**/target/` and keys that cache from `sources/**/Cargo.lock`.
- If a dependency fix should affect CI, confirm the relevant source's `Cargo.lock` resolved to the expected revision before changing the workflow.
- Keep source logs with a stable prefix such as `[montetai]`.
