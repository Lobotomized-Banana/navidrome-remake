#!/usr/bin/env bash
# Build the Rune frontend as a static WebAssembly site.
#
# Prerequisites (see README.md):
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version 0.2.129 --locked
#
# Usage: ./build-web.sh [--dev] [--out dist]
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
RUNE="$(cd "$HERE/../rune" && pwd)"
OUT="$HERE/dist"
PROFILE="release"
EXTRA=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dev) PROFILE="dev"; EXTRA+=("--dev"); shift ;;
    --out) OUT="$2"; shift 2 ;;
    *) echo "usage: $0 [--dev] [--out DIR]" >&2; exit 2 ;;
  esac
done

if ! rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown; then
  echo "wasm32-unknown-unknown target is missing: rustup target add wasm32-unknown-unknown" >&2
  exit 1
fi
if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "wasm-bindgen is missing: cargo install wasm-bindgen-cli --version 0.2.129 --locked" >&2
  exit 1
fi

cd "$HERE"
if [[ "$PROFILE" == "release" ]]; then
  cargo build --release --target wasm32-unknown-unknown -p navidrome-webui
  WASM="$HERE/target/wasm32-unknown-unknown/release/navidrome_webui.wasm"
  BINDGEN=(--remove-name-section)
else
  cargo build --target wasm32-unknown-unknown -p navidrome-webui
  WASM="$HERE/target/wasm32-unknown-unknown/debug/navidrome_webui.wasm"
  BINDGEN=()
fi

rm -rf "$OUT" "$OUT.next"
mkdir -p "$OUT" "$OUT.next"
wasm-bindgen --target web --out-dir "$OUT.next" --out-name app "${BINDGEN[@]}" "$WASM"

# Rune fetches the full Inter faces after the first frame; serve them next to app.js.
for face in Inter-Regular.ttf Inter-Medium.ttf Inter-SemiBold.ttf; do
  src="$RUNE/crates/rune/assets/web/$face"
  [[ -f "$src" ]] && cp "$src" "$OUT.next/"
done

cp "$HERE/index.html" "$OUT.next/index.html"
for entry in "$OUT.next"/*; do mv "$entry" "$OUT/"; done
rmdir "$OUT.next"

echo "Built $PROFILE web app in $OUT"
echo "Serve it with: python3 -m http.server 8000 -d $OUT"
echo "Then open http://localhost:8000 (localhost is allowed for WebGPU)."
