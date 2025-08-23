#!/usr/bin/env bash
set -eEuvx

function error_help()
{
    ERROR_MSG="It looks like something went wrong building the Example App Universal Binary."
    echo "error: ${ERROR_MSG}"
}
trap error_help ERR

# XCode tries to be helpful and overwrites the PATH. Reset that.
PATH="$(bash -l -c 'echo $PATH')"

# This should be invoked from inside xcode, not manually
if [[ "${#}" -ne 3 ]]
then
    echo "Usage (note: only call inside xcode!):"
    echo "path/to/build-scripts/xc-universal-binary.sh <FFI_TARGET> <SRC_ROOT_PATH> <buildvariant>"
    exit 1
fi
# what to pass to cargo build -p, e.g. logins_ffi
FFI_TARGET=${1}
# path to source code root
SRC_ROOT=${2}
# buildvariant from our xcconfigs
BUILDVARIANT=$(echo "${3}" | tr '[:upper:]' '[:lower:]')

RELFLAG=
if [[ "${BUILDVARIANT}" != "debug" ]]; then
    RELFLAG=--release
fi

# Note: We don't set LIBRARY_PATH to iOS SDK paths because that would interfere
# with proc macro compilation, which needs to run on the host system (macOS)

IS_SIMULATOR=0
if [ "${LLVM_TARGET_TRIPLE_SUFFIX-}" = "-simulator" ]; then
  IS_SIMULATOR=1
fi

TARGET_DIR="target"
BUILT_PRODUCTS_DIR="${SRCROOT}/TobogganApp"

# Ensure the destination directory exists
mkdir -p "${BUILT_PRODUCTS_DIR}"

# Change to the correct working directory
cd "${SRC_ROOT}"

# The actual library name is based on the lib.name in Cargo.toml, not the package name
LIB_NAME="toboggan"

# Build Rust library for all architectures first, then generate Swift bindings
# This ensures the bindings are generated after all compilation is complete

UDL_FILE="${SRC_ROOT}/src/toboggan.udl"
BINDINGS_DIR="${SRCROOT}/TobogganApp"

# Ensure the bindings directory exists
mkdir -p "${BINDINGS_DIR}"

echo "Building Rust library for all architectures..."

for arch in $ARCHS; do
  case "$arch" in
    x86_64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        echo "Building for x86_64, but not a simulator build. What's going on?" >&2
        exit 2
      fi

      # Intel iOS simulator
      export CFLAGS_x86_64_apple_ios="-target x86_64-apple-ios"
      $HOME/.cargo/bin/cargo rustc -p "${FFI_TARGET}" --lib --crate-type staticlib $RELFLAG --target x86_64-apple-ios
      
      RUST_LIB_PATH="${TARGET_DIR}/x86_64-apple-ios/$([[ "${BUILDVARIANT}" != "debug" ]] && echo "release" || echo "debug")/lib${LIB_NAME}.a"
      # Copy the built library to where Xcode expects it
      cp "$RUST_LIB_PATH" "${BUILT_PRODUCTS_DIR}/"
      # Also copy to project directory for linker search path
      cp "$RUST_LIB_PATH" "${SRCROOT}/TobogganApp/"
      ;;

    arm64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        # Hardware iOS targets
        # export CFLAGS_aarch64_apple_ios="-target aarch64-apple-ios"
        $HOME/.cargo/bin/cargo rustc -p "${FFI_TARGET}" --lib --crate-type staticlib $RELFLAG --target aarch64-apple-ios
        
        RUST_LIB_PATH="${TARGET_DIR}/aarch64-apple-ios/$([[ "${BUILDVARIANT}" != "debug" ]] && echo "release" || echo "debug")/lib${LIB_NAME}.a"
        # Copy the built library to where Xcode expects it
        cp "$RUST_LIB_PATH" "${BUILT_PRODUCTS_DIR}/"
        # Also copy to project directory for linker search path
        cp "$RUST_LIB_PATH" "${SRCROOT}/TobogganApp/"
      else
        # M1 iOS simulator
        # export CFLAGS_aarch64_apple_ios_sim="-target aarch64-apple-ios-simulator"
        $HOME/.cargo/bin/cargo rustc -p "${FFI_TARGET}" --lib --crate-type staticlib $RELFLAG --target aarch64-apple-ios-sim
        
        RUST_LIB_PATH="${TARGET_DIR}/aarch64-apple-ios-sim/$([[ "${BUILDVARIANT}" != "debug" ]] && echo "release" || echo "debug")/lib${LIB_NAME}.a"
        # Copy the built library to where Xcode expects it
        cp "$RUST_LIB_PATH" "${BUILT_PRODUCTS_DIR}/"
        # Also copy to project directory for linker search path
        cp "$RUST_LIB_PATH" "${SRCROOT}/TobogganApp/"

        # Generate Swift bindings using the EXACT same library we just compiled
        RUST_UDL_PATH="${SRC_ROOT}/toboggan-ios/src/toboggan.udl"
        echo "🍎 Generate for sim: $RUST_UDL_PATH (using compiled library metadata)"
        $HOME/.cargo/bin/cargo run $RELFLAG -p "${FFI_TARGET}" --bin uniffi-bindgen -- generate --library --language swift --out-dir "${SRCROOT}/TobogganApp/" "$RUST_LIB_PATH"
      fi
  esac
done

echo "All architectures built successfully! $ARCHS - $IS_SIMULATOR"
# Swift bindings are now handled by Xcode UDL Build Rule

echo "Build script completed - Rust library built and ready for UDL Build Rule to generate Swift bindings"
