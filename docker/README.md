# TrueNAS deployment

## What I found online (TrueNAS Community Edition 25.10 / 26)

- **Apps → Discover → Custom App** opens a guided wizard for a single container image
  (name/version, image + tag, entrypoint/env, security context, network/ports, portal, storage, resources).
- The sibling **Install via YAML** action opens the **Add Custom App** screen: paste a Docker
  Compose file into **Custom Config**. The file must begin with a top-level `name:` or `services:`
  key. TrueNAS only does basic YAML validation — you own the configuration.
- Compose `build.context` may be a **Git repository URL**, so TrueNAS can build the image from
  source with no registry and no SSH. This is the easiest all-web-UI route.
- App names must be lowercase alphanumeric, may contain `-`, and cannot start/end with `-`.
- Create the datasets/folders **before** installing. The default TrueNAS apps user is `568:568`
  (volume mount `ixVolume` is for testing; host paths are recommended for production).
- Avoid host ports 80/443 (TrueNAS UI). We use **4533**.
- The image's own healthcheck covers readiness; no extra sidecar is needed.

## How this image is built

One container is enough. The Dockerfile:

1. Copies the fork into a Go builder, then copies `webui-rune/dist/` (the compiled Rune/WASM
   frontend) into `navidrome-remake/ui/build/`.
2. Replaces `index.html` with `docker/index.gohtml`, which lets Navidrome inject
   `window.__APP_CONFIG__` (used for first-run admin creation and the default Server URL).
3. Builds the Go binary with the required tags (`netgo,sqlite_fts5`, plus `nodynamic` on 32-bit).
4. Produces a small Alpine runtime with `curl`, `ffmpeg` and `sqlite`.

Navidrome's existing `//go:embed all:build` serves the Rune app at `/app/`; `/` redirects there.
No nginx/Caddy/proxy is required. The frontend was built with Rune's `webgl` feature, so it works
even over plain HTTP on a LAN (WebGPU is only available in secure contexts).

## Option A — prebuilt image from GHCR (recommended, fastest install)

1. **Publish the source** (one time):
   ```sh
   cd /home/Andrew/Projects/Navidrome-Remake
   ./docker/prepare-source.sh /tmp/navidrome-remake-repo
   cd /tmp/navidrome-remake-repo
   git init && git add . && git commit -m "Navidrome Remake"
   git remote add origin https://github.com/YOUR_USER/navidrome-remake.git
   git push -u origin main
   ```
   Keep the repo **public** so TrueNAS can pull the image without credentials.
2. The included `.github/workflows/docker-publish.yml` runs on every push to `main` and publishes
   `ghcr.io/YOUR_USER/navidrome-remake:latest` (check **Settings → Actions → General → Workflow
   permissions → Read and write** if the first run cannot push packages).
3. In TrueNAS: **Apps → Discover → Custom App → Install via YAML**.
   Name: `navidrome-remake`. Paste `docker/docker-compose.truenas.yaml` after replacing:
   - `YOUR_GITHUB_USER` → your GitHub username
   - `/mnt/YOUR_POOL/...` → your datasets
   - `TZ` → your timezone
4. Click **Save**, then open **http://TRUENAS_IP:4533/**.

## Option B — build from GitHub directly in TrueNAS (no registry)

1. Push the source as in Option A step 1.
2. Paste `docker/docker-compose.truenas-build.yaml` into **Install via YAML**, replacing
   `YOUR_USER/YOUR_REPO`, the dataset paths and `TZ`.
3. TrueNAS clones the repo and builds the image on the box. The first install takes a few minutes;
   later installs reuse the build cache.

## Option C — local image tar (fallback, needs SSH)

`docker/navidrome-remake-local.tar` is a ready-to-load image built from this exact source.
```sh
docker load -i /path/to/docker/navidrome-remake-local.tar
```
Then use a compose file with `image: navidrome-remake:local` instead of the GHCR image. This is
**not** a web-UI-only workflow — Options A/B are.

## Datasets and permissions

Create two datasets before installing, for example:

- `YOUR_POOL/music` — your existing music library
- `YOUR_POOL/apps/navidrome-remake` — database, cache and config (`/data`)

With `user: "568:568"` in the Compose file, both must be writable by UID/GID 568. From the TrueNAS
UI: **Datasets → (dataset) → Edit Permissions**, or use the TrueNAS CLI:
```sh
chown -R 568:568 /mnt/YOUR_POOL/apps/navidrome-remake
```
`/music` is mounted read-only (`:ro`). If you prefer running as root, delete the `user:` line.

## First run

Open `http://TRUENAS_IP:4533/`. It redirects to `/app/` and the Rune UI detects that no users
exist, so it shows **Create admin** instead of **Sign in**. Create the admin account, then point
the library at `/music` in Navidrome's settings. The **Server URL** field is prefilled from the
page origin; change it only if you access the app through a different hostname/base path.

## Verification performed here

- `docker build -f docker/Dockerfile -t navidrome-remake:local .` succeeded
- `/ping` returns 200, `/` redirects to `/app/`, `/app/app_bg.wasm` is served as `application/wasm`
- The embedded UI was opened in Chromium from the running container with no console errors
- On a fresh data volume, the injected config has `firstTime: true` and the UI shows the
  **Create admin** screen; `POST /auth/createAdmin` returns a working token/salt pair
- `webui-rune` native tests (3) and the full Go test suite pass

## Notes / limitations

- The frontend needs WebGPU or WebGL2. The image is built with Rune's `webgl` fallback; use a
  current Chrome/Edge/Firefox browser.
- This is a remake/fork, not the official Navidrome image. Pin a specific tag rather than `latest`
  if you want reproducible upgrades; the workflow publishes commit SHAs and tags too.
- Navidrome's React UI is replaced by the Rune UI. The Go API, scanner and Subsonic endpoints are
  unchanged, so standard Subsonic clients still work on port 4533.
