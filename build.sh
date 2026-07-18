#!/usr/bin/env bash
# Build the Bevy game to WebAssembly and stage it under web/pkg for Vercel.
set -euo pipefail

# Cargo replaces '-' with '_' in the emitted artifact name.
CRATE=zombies_bevy
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

# Optional size optimisation if wasm-opt is available.
if command -v wasm-opt >/dev/null 2>&1; then
  echo "==> wasm-opt -Oz"
  wasm-opt -Oz -o "$OUT/zombies_bevy_bg.wasm" "$OUT/zombies_bevy_bg.wasm"
fi

echo "==> done. Serve ./web or deploy with Vercel."
ls -lh "$OUT"
