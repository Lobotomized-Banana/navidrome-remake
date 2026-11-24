# syntax=docker/dockerfile:1
#
# Navidrome Remake — single-container image.
#
# The Go server embeds the Rune web UI: the prebuilt static bundle from
# webui-rune/dist is copied into ui/build before `go build`, and Navidrome's
# existing `//go:embed all:build` serves it at /app/. No second web server,
# nginx, or reverse proxy is needed.
#
# Build context must be the Navidrome-Remake root:
#   docker build -f docker/Dockerfile -t navidrome-remake:local .

FROM --platform=$BUILDPLATFORM golang:1.27-alpine AS build
RUN apk add --no-cache gcc musl-dev zlib-dev
WORKDIR /src

# Backend source (the fork, including the remake changes).
COPY navidrome-remake/ ./

# Prebuilt Rune WebAssembly frontend. Rebuild it with webui-rune/build-web.sh
# when the UI source changes; the checked-in dist/ is used here so the image
# build needs only Go, not the Rust/wasm toolchain.
COPY webui-rune/dist/ ./ui/build/
# Replace the static page with the Go-templated one so Navidrome injects
# __APP_CONFIG__ (firstTime, baseURL) and the Rune UI can offer first-run admin
# creation and default the Server URL correctly.
COPY docker/index.gohtml ./ui/build/index.html

RUN go mod download

ARG GIT_SHA=unknown
ARG GIT_TAG=dev
RUN set -e; \
    BUILD_TAGS="$(./release/build-tags.sh)"; \
    echo "Building with tags: $BUILD_TAGS"; \
    CGO_ENABLED=1 go build \
      -tags="$BUILD_TAGS" \
      -ldflags="-w -s \
        -X github.com/navidrome/navidrome/consts.gitSha=${GIT_SHA} \
        -X github.com/navidrome/navidrome/consts.gitTag=${GIT_TAG}" \
      -o /out/navidrome .

############################################################################
FROM alpine:3.22 AS final
LABEL org.opencontainers.image.title="Navidrome Remake"
LABEL org.opencontainers.image.description="Navidrome fork with improved album identification and a Rune WebAssembly web UI"
LABEL org.opencontainers.image.source="https://github.com/navidrome/navidrome"

# curl: health check. ffmpeg: transcoding. sqlite + libwebp: runtime libraries
# used by the Go binary (purego/dlopen).
RUN apk add -U --no-cache curl ffmpeg sqlite libwebp libwebpdemux libwebpmux && \
    for lib in libwebp libwebpdemux libwebpmux; do \
        target=$(ls /usr/lib/$lib.so.* 2>/dev/null | head -1) && \
        [ -n "$target" ] && ln -sf "$target" /usr/lib/$lib.so; \
    done

COPY --from=build /out/navidrome /app/navidrome

VOLUME ["/data", "/music"]
ENV ND_MUSICFOLDER=/music \
    ND_DATAFOLDER=/data \
    ND_CONFIGFILE=/data/navidrome.toml \
    ND_PORT=4533 \
    TZ=Etc/UTC

EXPOSE 4533
WORKDIR /app
ENV PATH="/app:${PATH}"

HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl --silent --fail http://127.0.0.1:${ND_PORT}/ping > /dev/null || exit 1

ENTRYPOINT ["/app/navidrome"]
