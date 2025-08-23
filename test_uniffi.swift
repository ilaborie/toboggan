#!/usr/bin/env swift

// Simple test to verify UniFFI checksum without full iOS app
// Compile with: swiftc -L. -ltoboggan_ios_core -import-objc-header tobogganFFI.h test_uniffi.swift

import Foundation

// Direct FFI function declarations
@_silgen_name("uniffi_toboggan_ios_core_checksum_func_create_client")
func uniffi_toboggan_ios_core_checksum_func_create_client() -> UInt16

@_silgen_name("ffi_toboggan_ios_core_uniffi_contract_version")
func ffi_toboggan_ios_core_uniffi_contract_version() -> UInt32

print("🔍 Testing UniFFI Integration...")
print("================================================")

// Check contract version
let contractVersion = ffi_toboggan_ios_core_uniffi_contract_version()
print("✅ Contract Version: \(contractVersion)")
print("   Expected: 29")
print("   Match: \(contractVersion == 29 ? "✅" : "❌")")
print()

// Check create_client checksum
let checksum = uniffi_toboggan_ios_core_checksum_func_create_client()
print("✅ create_client Checksum: \(checksum)")
print("   Expected: 14851")
print("   Match: \(checksum == 14851 ? "✅" : "❌")")
print()

// Print diagnostics
if checksum != 14851 {
    print("❌ CHECKSUM MISMATCH DETECTED!")
    print("   The library was compiled with a different UniFFI version or source")
    print("   than the Swift bindings expect.")
    print()
    print("🔧 Solutions:")
    print("   1. Ensure library and bindings were generated together")
    print("   2. Clean and rebuild: rm -rf target && cargo build --release")
    print("   3. Regenerate bindings: cargo run --bin uniffi-bindgen")
} else {
    print("✅ Checksums match! UniFFI integration is correct.")
}