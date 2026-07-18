#!/usr/bin/env bash
# Build the Bevy game to WebAssembly and stage it under web/pkg for Vercel.
set -euo pipefail

# The binary target keeps the package name (dash preserved).
CRATE=zombies-bevy
OUT=web/pkg

echo "==> cargo build (release, wasm32)"
cargo build --release --target wasm32-unknown-unknown

echo "==> wasm-bindgen"
rm -rf "$OUT"
wasm-bindgen \
  --no-typescript \
  --target web \
  --out-dir "$OUT" \
  --out-name zombies_bevy \
  "target/wasm32-unknown-unknown/release/${CRATE}.wasm"

# Optional size optimisation. NOTE: binaryen must be recent (>= ~v116). Older
# wasm-opt (e.g. v108) miscompiles wasm-bindgen's externref table and the module
# fails at init with "WebAssembly.Table.grow(): failed to grow table". If in doubt
# skip this — the CDN/host serves the wasm brotli-compressed anyway (~7MB on the
# wire). Enable by exporting WASM_OPT=1 with a modern wasm-opt on PATH.
if [ "${WASM_OPT:-0}" = "1" ] && command -v wasm-opt >/dev/null 2>&1; then
  echo "==> wasm-opt -Oz (reference-types)"
  wasm-opt -Oz --enable-reference-types --enable-bulk-memory \
    -o "$OUT/zombies_bevy_bg.wasm" "$OUT/zombies_bevy_bg.wasm"
fi

echo "==> done. Serve ./web or deploy with Vercel."
ls -lh "$OUT"
