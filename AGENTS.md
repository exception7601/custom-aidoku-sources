## Sources

- `sources/pt_BR.montetaiscanlator`

## Folder overview

- `sources/` contains source projects.
- `public/` contains generated source list artifacts.
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

Package the source.

```sh
env -C sources/pt_BR.montetaiscanlator aidoku package
```

Run offline source tests.

```sh
env -C sources/pt_BR.montetaiscanlator cargo test
```

Run integration source tests against live site data.
Do not use these tests to gate releases.

```sh
env -C sources/pt_BR.montetaiscanlator cargo test --features integration-tests
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

- `cargo test` is the offline suite and is safe for release gating.
- `cargo test --features integration-tests` hits the live site and is for manual verification only.
- Keep source logs with a stable prefix such as `[montetai]`.
