#!/bin/bash

set -e

# Configuration
TARGET_DIR="target"
XCFRAMEWORK_NAME="TobogganCore"
SWIFT_BINDINGS_DIR="$TARGET_DIR/swift"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf "$TARGET_DIR"
mkdir -p "$TARGET_DIR"

# Build for iOS targets
echo "🔨 Building Rust library for iOS targets..."

# Add iOS targets if not already installed
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim

# Build for iOS device (arm64)
echo "📱 Building for iOS device (aarch64-apple-ios)..."
cargo build --release --target aarch64-apple-ios

# Build for iOS simulator (x86_64 and arm64)
echo "🖥️ Building for iOS Simulator (x86_64-apple-ios)..."
cargo build --release --target x86_64-apple-ios

echo "🖥️ Building for iOS Simulator (aarch64-apple-ios-sim)..."
cargo build --release --target aarch64-apple-ios-sim

# Generate Swift bindings
echo "🔗 Generating Swift bindings..."
mkdir -p "$SWIFT_BINDINGS_DIR"

# Generate Swift bindings using cargo uniffi-bindgen
cargo uniffi-bindgen generate \
    --library target/aarch64-apple-ios/release/libtoboggan_ios_core.a \
    --language swift \
    --out-dir "$SWIFT_BINDINGS_DIR"

# Create universal library for simulator
echo "🔄 Creating universal simulator library..."
lipo -create \
    target/x86_64-apple-ios/release/libtoboggan_ios_core.a \
    target/aarch64-apple-ios-sim/release/libtoboggan_ios_core.a \
    -output target/libtoboggan_ios_core_sim.a

# Create XCFramework structure
echo "📦 Creating XCFramework..."
XCFRAMEWORK_DIR="$TARGET_DIR/$XCFRAMEWORK_NAME.xcframework"
mkdir -p "$XCFRAMEWORK_DIR"

# iOS device framework
IOS_FRAMEWORK="$XCFRAMEWORK_DIR/ios-arm64"
mkdir -p "$IOS_FRAMEWORK"
cp target/aarch64-apple-ios/release/libtoboggan_ios_core.a "$IOS_FRAMEWORK/"
cp -r "$SWIFT_BINDINGS_DIR"/* "$IOS_FRAMEWORK/"

# iOS simulator framework  
SIM_FRAMEWORK="$XCFRAMEWORK_DIR/ios-arm64_x86_64-simulator"
mkdir -p "$SIM_FRAMEWORK"
cp target/libtoboggan_ios_core_sim.a "$SIM_FRAMEWORK/libtoboggan_ios_core.a"
cp -r "$SWIFT_BINDINGS_DIR"/* "$SIM_FRAMEWORK/"

# Create Info.plist for XCFramework
cat > "$XCFRAMEWORK_DIR/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>AvailableLibraries</key>
    <array>
        <dict>
            <key>LibraryIdentifier</key>
            <string>ios-arm64</string>
            <key>LibraryPath</key>
            <string>libtoboggan_ios_core.a</string>
            <key>SupportedArchitectures</key>
            <array>
                <string>arm64</string>
            </array>
            <key>SupportedPlatform</key>
            <string>ios</string>
        </dict>
        <dict>
            <key>LibraryIdentifier</key>
            <string>ios-arm64_x86_64-simulator</string>
            <key>LibraryPath</key>
            <string>libtoboggan_ios_core.a</string>
            <key>SupportedArchitectures</key>
            <array>
                <string>arm64</string>
                <string>x86_64</string>
            </array>
            <key>SupportedPlatform</key>
            <string>ios</string>
            <key>SupportedPlatformVariant</key>
            <string>simulator</string>
        </dict>
    </array>
    <key>CFBundlePackageType</key>
    <string>XFWK</string>
    <key>XCFrameworkFormatVersion</key>
    <string>1.0</string>
</dict>
</plist>
EOF

echo "✅ Build complete!"
echo "📁 XCFramework created at: $XCFRAMEWORK_DIR"
echo "🔗 Swift bindings available in: $SWIFT_BINDINGS_DIR"

# Test the Swift bindings generation
echo "🧪 Testing Swift bindings compilation..."
cd "$SWIFT_BINDINGS_DIR"
if command -v swiftc &> /dev/null; then
    swiftc -parse *.swift
    echo "✅ Swift bindings compiled successfully"
else
    echo "⚠️ swiftc not found, skipping Swift compilation test"
fi