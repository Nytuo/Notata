# Flatpak packaging

`fr.nytuo.notata.yml` builds Notata from source inside the Flatpak sandbox,
the way Flathub requires: no network access at build time, every cargo crate and
npm package is listed in `cargo-sources.json` / `node-sources.json`.

The Flatpak build uses npm, not bun: keep `package-lock.json` in sync with
`package.json` (`npm install --package-lock-only`) when dependencies change.
It must carry `resolved`/`integrity` fields — the offline generator cannot
download packages without them.

## Regenerate the offline sources

Whenever `src-tauri/Cargo.lock` or `package-lock.json` changes (the
`flatpak-check` workflow fails when they are stale):

```sh
python3 -m venv .venv && . .venv/bin/activate
pip install aiohttp toml tomlkit "git+https://github.com/flatpak/flatpak-builder-tools.git#subdirectory=node"
curl -O https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
python3 flatpak-cargo-generator.py src-tauri/Cargo.lock -o flatpak/cargo-sources.json
python3 -m flatpak_node_generator npm package-lock.json -o flatpak/node-sources.json
```

## Build locally

```sh
flatpak install flathub org.gnome.Sdk//50 org.gnome.Platform//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08 org.freedesktop.Sdk.Extension.node24//25.08
flatpak-builder --force-clean --user --install build-dir flatpak/fr.nytuo.notata.yml
flatpak run fr.nytuo.notata
```

## Publishing to Flathub

The local manifest builds the working tree (`type: dir`). Flathub needs a
pinned source, so `flathub_sync.py` swaps the `type: dir` source for a git
source at a release tag, and adds the release to the
metainfo.

- **Updates** — `.github/workflows/flathub-publish.yml` runs when a GitHub
  Release is published and opens a pull request on
  [flathub/fr.nytuo.notata](https://github.com/flathub/fr.nytuo.notata).
  Flathub test-builds the PR; merge it to publish. Needs the `FLATHUB_TOKEN`
  secret (classic PAT, `public_repo` scope, from a maintainer of that repo).
- **Initial submission** — until Flathub accepts the app, that repo does not
  exist and the workflow only uploads the `flathub-files` artifact. Those files
  go in a branch of your fork of [flathub/flathub](https://github.com/flathub/flathub)
  based on `new-pr`, submitted as a pull request against `new-pr`.
