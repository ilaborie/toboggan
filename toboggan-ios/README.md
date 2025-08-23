# Toboggan iOS Rust Core

Rust implementation for iOS integration with Toboggan presentations. This directory contains the Rust source code that can be used for future real server connectivity.

## Status

🔄 **Future Integration**: This Rust code provides the foundation for real Toboggan server connectivity. The current iOS app (in `../TobogganApp/`) uses a pure Swift implementation with mock data.

## Overview

This Rust library provides:
- **WebSocket Client**: Real-time connection to Toboggan server
- **UniFFI Bindings**: Type-safe Swift integration
- **Command System**: Navigation and presentation control
- **Modular Architecture**: Clean separation of concerns

## Architecture

### Rust Modules
- `client.rs` - WebSocket client implementation
- `command.rs` - Presentation commands (Next, Previous, etc.)
- `config.rs` - Client configuration
- `error.rs` - Error handling
- `state.rs` - Presentation state management
- `types.rs` - Core data types (Slide, Talk, etc.)
- `lib.rs` - UniFFI exports and public API

### UniFFI Integration
The `toboggan.udl` file defines the interface for Swift bindings generation.

## Dependencies

- **toboggan-core**: Core domain models
- **toboggan-client**: Shared client library  
- **uniffi**: Rust-Swift interoperability
- **tokio**: Async runtime

## Building

```bash
# Build iOS framework (when needed in future)
./build.sh

# Or use mise task from workspace root
cd .. && mise build:ios
```

## Current Usage

The iOS app in `../TobogganApp/` currently uses a pure Swift implementation for simplicity. This Rust code provides the foundation for future real server integration.

### Integration Process (Future)
1. Build the Rust library with `./build.sh`
2. Replace mock implementations in Swift with real UniFFI types
3. Update `TobogganCore.swift` to use actual Rust-generated bindings
4. Configure WebSocket connection to real Toboggan server

## Development

- **Language**: Rust 2024 edition
- **Linting**: Comprehensive clippy rules enabled
- **Safety**: No unsafe code allowed
- **Testing**: Unit tests for all modules

## File Structure

```
toboggan-ios/
├── src/
│   ├── lib.rs              # Main exports and UniFFI setup
│   ├── client.rs           # WebSocket client
│   ├── command.rs          # Presentation commands
│   ├── config.rs           # Configuration types
│   ├── error.rs            # Error handling
│   ├── state.rs            # State management
│   ├── types.rs            # Core data types
│   ├── utils.rs            # Utility functions
│   ├── main.rs             # Binary stub
│   └── toboggan.udl        # UniFFI interface definition
├── Cargo.toml              # Rust dependencies
├── build.sh                # iOS build script
└── README.md               # This file
```

## Next Steps

When ready to integrate with real Toboggan server:
1. Test the Rust implementation
2. Generate Swift bindings with UniFFI
3. Replace mock data in iOS app
4. Add server configuration UI
5. Test real-time presentation control

The current pure Swift approach allows rapid UI development, while this Rust code provides the foundation for production server connectivity.