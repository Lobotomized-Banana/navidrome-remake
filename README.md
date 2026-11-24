# Navidrome Remake

This workspace contains a remake of [Navidrome](https://github.com/navidrome/navidrome) with
improved album identification, more robust file-metadata parsing, and a brand-new web interface
built with [Rune](https://github.com/anomalyco/rune).

## Layout

| Path | What it is |
| --- | --- |
| `navidrome/` | Upstream Navidrome reference clone (unmodified) |
| `rune/` | Upstream Rune framework reference clone (unmodified) |
| `navidrome-remake/` | The new backend — a fork of Navidrome on branch `remake/album-id-and-metadata` |
| `webui-rune/` | The new Rune-based web frontend (Rust, compiles to WebAssembly) |

## What changed

### Backend (`navidrome-remake/`)

- **Unicode-aware identity normalization** (`utils/str/identity.go`): NFC folding, accent
  folding, invisible-character stripping, quote/dash unification and whitespace collapsing, exposed
  as opt-in PID attributes (`albumartistid_norm`, `album_norm`, `title_norm`). The legacy
  attributes are untouched, so an existing Navidrome database can be reused without invalidating
  stored IDs.
- **Opt-in enhanced album PID** (`consts.EnhancedAlbumPID`): a configurable `PID.Album` spec that
  prefers a MusicBrainz **release-group** ID over a single release ID, then the album artist's
  MusicBrainz ID, then normalized names, with a **folder fallback** for untagged files. Changing a
  PID spec triggers a documented full rescan.
- **Various Artists detection** now recognizes `VA`, `Various Artist`, case/accent variants and
  the canonical MusicBrainz ID, not only an empty album artist plus `TCMP`.
- **Robust tag parsing**: `N/Total`, `N of M`, `N-M` and spaced variants for track/disc numbers;
  a malformed value no longer overwrites an explicit `tracktotal`. Boolean tags accept `yes/y/on`.
  Dates accept `2014.05.21`, `2014/05/21` and ISO timestamps, with calendar validation and
  rune-safe truncation. Duplicate tag values are deduplicated case-insensitively and trimmed.
- Tests added for every change; the **full upstream Go test suite passes**.

See [`navidrome-remake/REMAKE_CHANGES.md`](navidrome-remake/REMAKE_CHANGES.md) for details.

### Frontend (`webui-rune/`)

A new Rune app with login, artist/album browsing, album track lists, search, favorites, cover
art, and a playback bar. It talks to Navidrome's `/auth/login` and `/rest/*` Subsonic API through
a small `fetch()` bridge and the browser's `<audio>` element.

- `cargo test` — 3 headless UI tests pass
- `cargo run -- --snap out/albums.png --scene albums` — deterministic PNG renders
- `./build-web.sh` — produces a deployable static site in `dist/`
- The built site was served and verified to render in a real browser with no console errors.

See [`webui-rune/README.md`](webui-rune/README.md) for build/run instructions and limitations.

## TrueNAS / Docker

A single-container image and ready-to-paste TrueNAS Compose files are in [`docker/`](docker/README.md):

- `docker/docker-compose.truenas.yaml` — paste into **Apps → Custom App → Install via YAML** after
  publishing the image to GHCR with the included GitHub Actions workflow
- `docker/docker-compose.truenas-build.yaml` — builds from a GitHub repo directly on TrueNAS (no registry)
- `docker/navidrome-remake-local.tar` — prebuilt 74 MB image for `docker load` fallback
- `docker/Dockerfile` — builds the Go backend with the Rune WASM UI embedded; `/app/` is served by
  Navidrome itself, so no reverse proxy is needed

Verified in a real container: `/ping`, `/app/` redirect, `application/wasm` MIME type, browser render,
and the first-run **Create admin** flow.

## Quick start

```sh
# Backend (build flags are required by upstream's buildtags package)
cd navidrome-remake
go build -tags=netgo,sqlite_fts5 ./...
go test -tags=netgo,sqlite_fts5 ./...

# Frontend — native / headless
cd ../webui-rune
cargo test
cargo run -- --snap out/albums.png --scene albums

# Frontend — browser bundle
./build-web.sh
python3 -m http.server 8000 -d dist
```

To enable the enhanced album identity on a library, set its `PID.Album` (or `pid_album`) to the
value of `consts.EnhancedAlbumPID`:

```toml
PID.Album = "musicbrainz_releasegroupid|musicbrainz_albumid|albumartist_mbid,albumartistid_norm,album_norm,albumversion,releasedate|folder,album_norm,albumartistid_norm"
# optional matching track spec:
PID.Track = "musicbrainz_trackid|albumid,discnumber,tracknumber,title_norm"
```

Changing the PID spec forces a full rescan; existing annotations are migrated by Navidrome's
`albumIDMap` mechanism.
