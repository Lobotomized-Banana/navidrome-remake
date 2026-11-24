# Navidrome Remake — Rune web interface

A replacement web frontend for Navidrome built with [Rune](../../rune), a GPU-rendered Rust UI
framework that compiles to WebAssembly. The same `view` function drives a native window and the
browser app.

## Features

- Server/username/password login against `POST /auth/login`, and first-run admin creation against
  `POST /auth/createAdmin` when Navidrome reports no users (config injected into `index.html` by the Go server)
- Server URL defaults to the current page origin/base path from `window.__APP_CONFIG__`
- Subsonic API client (`getArtists`, `getAlbumList2`, `getArtist`, `getAlbum`, `search3`,
  `getStarred2`, `getCoverArt`, `stream`) with salt/token authentication
- Artist → albums → tracks browsing, search, favorites, back navigation
- Cover-art thumbnails via Rune's `ImageCache::url`
- Playback through a browser `<audio>` element (Rune has no web audio backend)
- Deterministic demo data on native builds so the UI can be rendered headlessly

## Layout

| File | Purpose |
| --- | --- |
| `src/lib.rs` | State, `view`, navigation, scenes, app entry point, headless tests |
| `src/api.rs` | Subsonic models, fetch bridge, URL builders, audio bridge, demo data |
| `src/main.rs` | Native binary wrapper |
| `index.html` | Custom page: WebGPU check, wasm bootstrap, hidden `<audio id="nd-audio">` |
| `build-web.sh` | Builds the static site into `dist/` |
| `out/*.png` | Headless snapshots of the login, album-list and album-detail scenes |

## Commands

```sh
# Native checks
cargo check
cargo test
cargo run -- --snap out/albums.png --scene albums
cargo run -- --snap out/artists.png --scene artists
cargo run -- --snap out/album.png --scene album
cargo run -- --snap out/login.png --scene login

# Browser bundle (needs the wasm target and wasm-bindgen CLI)
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
./build-web.sh
python3 -m http.server 8000 -d dist
# open http://localhost:8000
```

The wasm-bindgen version is pinned to the one in Rune's lockfile. `dist/` contains
`index.html`, `app.js`, `app_bg.wasm`, the wasm-bindgen `snippets/` folder and the Inter fonts —
serve the whole folder, with `.wasm` as `application/wasm`. Production needs HTTPS; localhost
is allowed for WebGPU during development.

## Connecting to Navidrome

Enter the server's origin in the **Server URL** field (for example `http://localhost:4533`;
include the configured base path if any). Navidrome's CORS middleware allows all origins, but
cookies are not used — authentication travels in the query string via the salt/token returned by
`/auth/login`. Cover-art requests include the same credentials, so cross-origin images work
without a proxy.

## Verification performed

- `cargo check` (native) and `cargo check --target wasm32-unknown-unknown` — clean
- `cargo test` — 3 headless UI tests pass
- Headless PNG snapshots for the login, albums and album-detail scenes (in `out/`)
- `./build-web.sh` — produced `dist/` (about 10 MB unoptimized wasm)
- The built site was served and opened in Chromium: the login screen rendered with no console
  errors and the wasm/snippets/font requests all returned 200

## Limitations

- Rune's web support is experimental. This app enables Rune's `webgl` feature, so it falls back to
  WebGL2 when WebGPU is unavailable (e.g. plain HTTP on a LAN).
- Rune has no in-framework HTTP client or web audio; both are bridged through browser JS.
- The framework's own note applies: one app per page.
- Playback uses the plain `/rest/stream` endpoint; the OpenSubsonic transcoding-negotiation flow
  used by Navidrome's React UI is not implemented yet.
- `wasm-opt` was not available, so the bundle is unoptimized. Run `wasm-opt -Oz` on
  `dist/app_bg.wasm` for production.
