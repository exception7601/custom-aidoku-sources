# `AGENT.md`

This guide is only for `pt_BR.montetaiscanlator`.
Use it as the default workflow for this source.

## Source

- `sources/pt_BR.montetaiscanlator`

## Folder overview

- `sources/` contains source projects.
- `public/` contains generated source list artifacts.
- `public/index.json` and `public/index.min.json` are the list entrypoints.
- `public/sources/` contains packaged `.aix` outputs.
- `public/icons/` contains icon assets copied during list build.

## Commands for sources 

Package the source.

```sh
env -C sources/pt_BR.montetaiscanlator aidoku package
```

Run source tests.

```sh
env -C sources/pt_BR.montetaiscanlator cargo test
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
2. Run `env -C sources/pt_BR.montetaiscanlator aidoku package`.
3. Run `aidoku verify sources/pt_BR.montetaiscanlator/package.aix`.
4. Run `aidoku build sources/*/package.aix --name "Aidoku Custom Sources"`.

## Notes

- Keep source logs with a stable prefix such as `[montetai]`.
