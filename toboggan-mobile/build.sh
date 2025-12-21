#!/bin/bash

set -eEuvx

# Configuration
TARGET_DIR="target"
SWIFT_BINDINGS_DIR="$TARGET_DIR/swift"

# Clean only what needs refreshing (preserve Rust incremental compilation)
echo "🧹 Cleaning Swift bindings and XCFramework..."
rm -rf "$SWIFT_BINDINGS_DIR"

# Build for iOS targets
echo "🔨 Building Rust library for iOS targets..."

# Add iOS targets if not already installed
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim

# Build for iOS device (arm64)
echo "📱 Building for iOS device (aarch64-apple-ios)..."
cargo build --quiet --release --target aarch64-apple-ios

# Build for iOS simulator (x86_64 and arm64)
echo "🖥️ Building for iOS Simulator (x86_64-apple-ios)..."
cargo build --quiet --release --target x86_64-apple-ios

echo "🖥️ Building for iOS Simulator (aarch64-apple-ios-sim)..."
cargo build --quiet --release --target aarch64-apple-ios-sim

# Generate Swift bindings
echo "🔗 Generating Swift bindings..."
mkdir -p "$SWIFT_BINDINGS_DIR"

# Build uniffi-bindgen for host system first
echo "🔧 Building uniffi-bindgen for host system..."
cargo build --release --bin uniffi-bindgen

# # CRITICAL: Generate from EXACT SAME library that will be linked
# # This ensures scaffolding and library have matching checksums
# cargo run --release --bin uniffi-bindgen -- \
#     generate \
#     --library target/aarch64-apple-ios/release/libtoboggan.a \
#     --language swift \
#     --out-dir "$SWIFT_BINDINGS_DIR"

# # Create universal library for simulator
# echo "🔄 Creating universal simulator library..."
# lipo -create \
#     target/x86_64-apple-ios/release/libtoboggan.a \
#     target/aarch64-apple-ios-sim/release/libtoboggan.a \
#     -output target/libtoboggan_sim.a

# # Note: XCFramework creation removed - using individual files approach per Mozilla UniFFI pattern

echo "✅ Build complete!"
echo "🔗 Swift bindings available in: $SWIFT_BINDINGS_DIR"
# echo "📱 Universal simulator library: target/aarch64-apple-ios-sim/release/libtoboggan.a"
echo "📱 iOS device library:          target/aarch64-apple-ios/release/libtoboggan.a"

# Test the Swift bindings generation
echo "🧪 Testing Swift bindings compilation..."
cd "$SWIFT_BINDINGS_DIR"
if command -v swiftc &> /dev/null; then
    swiftc -parse *.swift
    echo "✅ Swift bindings compiled successfully"
else
    echo "⚠️ swiftc not found, skipping Swift compilation test"
fi