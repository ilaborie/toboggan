#!/usr/bin/env bash
set -eEuv

function error_help()
{
    ERROR_MSG="Something went wrong building the Toboggan Rust library for iOS."
    echo "error: ${ERROR_MSG}"
}
trap error_help ERR

# XCode tries to be helpful and overwrites the PATH. Reset that.
PATH="$(bash -l -c 'echo $PATH')"

# This should be invoked from inside xcode, not manually
if [[ "${#}" -ne 3 ]]
then
    echo "Usage (note: only call inside xcode!):"
    echo "TobogganApp/xc-universal-binary.sh <FFI_TARGET> <SRC_ROOT_PATH> <buildvariant>"
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

# Where the static library and the generated bindings both go: the group Xcode
# compiles, and the linker search path. Deliberately *not* called
# `BUILT_PRODUCTS_DIR` — Xcode exports a variable of that name meaning its own
# build directory, and this script used to shadow it.
ARTIFACTS_DIR="${SRCROOT}/TobogganApp"
mkdir -p "${ARTIFACTS_DIR}"

cd "${SRC_ROOT}"

# The actual library name is based on the lib.name in Cargo.toml, not the package name
LIB_NAME="toboggan"

# Build Rust library for all architectures first, then generate Swift bindings
# This ensures the bindings are generated after all compilation is complete

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
      cp "$RUST_LIB_PATH" "${ARTIFACTS_DIR}/"
      ;;

    arm64)
      if [ $IS_SIMULATOR -eq 0 ]; then
        # Hardware iOS targets
        # export CFLAGS_aarch64_apple_ios="-target aarch64-apple-ios"
        $HOME/.cargo/bin/cargo rustc -p "${FFI_TARGET}" --lib --crate-type staticlib $RELFLAG --target aarch64-apple-ios

        RUST_LIB_PATH="${TARGET_DIR}/aarch64-apple-ios/$([[ "${BUILDVARIANT}" != "debug" ]] && echo "release" || echo "debug")/lib${LIB_NAME}.a"
        cp "$RUST_LIB_PATH" "${ARTIFACTS_DIR}/"
      else
        # M1 iOS simulator
        # export CFLAGS_aarch64_apple_ios_sim="-target aarch64-apple-ios-simulator"
        $HOME/.cargo/bin/cargo rustc -p "${FFI_TARGET}" --lib --crate-type staticlib $RELFLAG --target aarch64-apple-ios-sim

        RUST_LIB_PATH="${TARGET_DIR}/aarch64-apple-ios-sim/$([[ "${BUILDVARIANT}" != "debug" ]] && echo "release" || echo "debug")/lib${LIB_NAME}.a"
        cp "$RUST_LIB_PATH" "${ARTIFACTS_DIR}/"

      fi
  esac
done

echo "All architectures built successfully! $ARCHS - $IS_SIMULATOR"

# Generate the Swift bindings from the library that was just compiled.
#
# This used to sit inside the arm64-simulator branch of the loop above, so a
# device build kept whatever bindings the last simulator build happened to leave
# behind. UniFFI's checksum guard turns that mismatch into a `fatalError` at
# launch, on the device, which is the worst place to find out.
if [ -z "${RUST_LIB_PATH:-}" ]; then
  echo "No library was built for ${ARCHS}; cannot generate bindings." >&2
  exit 2
fi
echo "🍎 Generating Swift bindings from ${RUST_LIB_PATH}"
$HOME/.cargo/bin/cargo run $RELFLAG -p "${FFI_TARGET}" --bin uniffi-bindgen -- \
  generate --library --language swift --out-dir "${ARTIFACTS_DIR}/" "$RUST_LIB_PATH"

echo "Build script completed - Rust library built and Swift bindings generated"
