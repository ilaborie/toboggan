#!/bin/bash

# Build script for minimal WASM size
set -e

echo "Building Toboggan WASM with size optimizations..."

# Install wasm-pack if not available
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    cargo install wasm-pack
fi

# Build with release profile and target web
wasm-pack build --target web --release --out-dir pkg

# Further optimize the WASM file if wasm-opt is available
if command -v wasm-opt &> /dev/null; then
    echo "Optimizing WASM with wasm-opt..."
    wasm-opt -Os -o pkg/toboggan_wasm_bg.wasm pkg/toboggan_wasm_bg.wasm
else
    echo "wasm-opt not found, skipping additional optimization"
    echo "Install with: npm install -g wasm-opt"
fi

# Show file size
echo "Build complete! File sizes:"
ls -lh pkg/toboggan_wasm_bg.wasm
echo "Gzipped size:"
gzip -c pkg/toboggan_wasm_bg.wasm | wc -c | awk '{print $1/1024 " KB"}'

echo "To serve locally: python3 -m http.server 8000"