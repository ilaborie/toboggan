# Toboggan iOS Rust Core

Rust implementation providing the core functionality for iOS integration with Toboggan presentations. This crate serves as the bridge between the Toboggan ecosystem and native iOS applications through UniFFI bindings.

## Status & Integration Strategy

🔄 **Future Integration Phase**: This Rust code provides a complete, production-ready foundation for real Toboggan server connectivity. The current iOS app (in `../TobogganApp/`) uses a pure Swift implementation with mock data to enable rapid UI development and iteration.

**Current Approach Benefits:**
- Fast SwiftUI development cycles without Rust compilation delays
- Immediate SwiftUI previews and hot-reload functionality
- Independent iOS UI development and testing
- Easy onboarding for Swift developers

**Future Integration Benefits:**
- Shared business logic with other Toboggan clients
- Real-time WebSocket synchronization
- Type-safe communication via UniFFI
- Consistent presentation behavior across platforms

## Overview

This Rust library provides a complete iOS-specific implementation of the Toboggan client with:

### Core Functionality
- **WebSocket Client**: Async connection to Toboggan server with automatic reconnection
- **Presentation Control**: Full command system (Next, Previous, Play, Pause, etc.)
- **State Management**: Thread-safe presentation state synchronization
- **Error Handling**: Comprehensive error types with user-friendly messages

### iOS Integration
- **UniFFI Bindings**: Automatic Swift interface generation
- **Type Safety**: Strong typing across the Rust-Swift boundary
- **Async Support**: Native async/await patterns for iOS
- **Memory Management**: Automatic reference counting integration

### Network Features
- **Connection Management**: Robust WebSocket handling with retry logic
- **Command Queue**: Offline command buffering and synchronization
- **State Reconciliation**: Automatic state sync when reconnecting
- **Error Recovery**: Graceful handling of network interruptions

## Architecture

### Core Design Principles
- **Single Responsibility**: Each module has a clear, focused purpose
- **Async-First**: All operations are non-blocking and reactive
- **Type Safety**: Extensive use of Rust's type system for correctness
- **Error Transparency**: Clear error propagation to Swift layer

### Module Structure

#### `client.rs` - WebSocket Client Implementation
```rust
pub struct TobogganClient {
    // WebSocket connection management
    // Command queue and state synchronization
    // Automatic reconnection logic
}

impl TobogganClient {
    pub async fn connect(&self, url: String) -> Result<()>;
    pub async fn send_command(&self, command: Command) -> Result<()>;
    pub fn subscribe_to_state(&self) -> StateStream;
}
```

#### `command.rs` - Presentation Commands
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    Next, Previous, First, Last,
    Goto { slide: SlideId },
    Play, Pause, Resume,
    Register { client_id: ClientId },
}
```

#### `state.rs` - Presentation State Management
```rust
#[derive(Clone, Debug)]
pub enum PresentationState {
    Disconnected,
    Connected { server_state: State },
    Error { message: String },
}
```

#### `config.rs` - Client Configuration
```rust
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub server_url: String,
    pub reconnect_interval: Duration,
    pub command_timeout: Duration,
}
```

#### `error.rs` - Comprehensive Error Handling
```rust
#[derive(Debug, thiserror::Error)]
pub enum TobogganError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),
    #[error("Invalid command: {0}")]
    CommandError(String),
    // ... other error variants
}
```

### UniFFI Integration

The `toboggan.udl` file defines the complete interface for Swift bindings:

```webidl
namespace toboggan {
    TobogganClient create_client(ClientConfig config);
};

interface TobogganClient {
    [Async]
    void connect(string url);
    
    [Async]  
    void send_command(Command command);
    
    StateStream subscribe_to_state();
};
```

**Benefits of UniFFI:**
- Automatic Swift binding generation
- Memory safety across language boundaries  
- Native Swift async/await support
- Automatic error handling conversion
- Type-safe data serialization

## Dependencies

- **toboggan-core**: Core domain models
- **toboggan-client**: Shared client library  
- **uniffi**: Rust-Swift interoperability
- **tokio**: Async runtime

## Building

### Prerequisites
- **Rust Toolchain**: Latest stable Rust with iOS targets
- **iOS Targets**: Required for cross-compilation
- **UniFFI**: For Swift binding generation

```bash
# Install iOS targets
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim

# Install UniFFI CLI (if needed)
cargo install uniffi_bindgen
```

### Build Process

#### Option 1: Using Mise (Recommended)
```bash
# From workspace root - builds for all iOS targets
mise build:ios
```

#### Option 2: Manual Build
```bash
# Navigate to iOS library directory
cd toboggan-mobile

# Build iOS framework with script
./build.sh

# This creates:
# - target/universal-ios/release/TobogganCore.xcframework
# - Generated Swift bindings in target/uniffi/
```

#### Option 3: Individual Target Build
```bash
# Build for specific targets
cargo build --target aarch64-apple-ios --release      # iOS device (ARM64)
cargo build --target x86_64-apple-ios --release       # iOS simulator (Intel)
cargo build --target aarch64-apple-ios-sim --release  # iOS simulator (Apple Silicon)
```

### Build Output

The build process generates:
```
target/
├── uniffi/
│   ├── TobogganCore.swift        # Swift interface
│   ├── TobogganCore.h            # C header
│   └── TobogganCore-Bridging-Header.h
├── universal-ios/
│   └── release/
│       └── TobogganCore.xcframework/  # Universal iOS framework
└── [target]/release/
    └── libtoboggan_ios.a         # Static library per target
```

## Current Development Approach

### Phase 1: Swift-Only Implementation (Current)
The iOS app in `../TobogganApp/` uses a pure Swift implementation with several advantages:

**Architecture:**
```swift
// Current: Pure Swift with mock types
protocol SlideProtocol { /* ... */ }
struct MockSlide: SlideProtocol { /* ... */ }

// ViewModels use mock types for rapid development
class PresentationViewModel: ObservableObject {
    @Published var slides: [MockSlide] = MockData.sampleSlides
}
```

**Benefits:**
- **Fast Iteration**: No Rust compilation delays during UI development
- **SwiftUI Previews**: Immediate preview support without dependencies
- **Easy Debugging**: Standard Swift debugging tools and workflow
- **Team Velocity**: Swift developers can work independently

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
toboggan-mobile/
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

## Integration Roadmap

### Phase 2: Rust Integration (Future)

When ready for production server connectivity:

#### Integration Steps

1. **Build Rust Framework**
   ```bash
   # Generate iOS framework and Swift bindings
   mise build:ios
   ```

2. **Update Xcode Project**
   ```bash
   # Add framework to iOS project
   cp target/universal-ios/release/TobogganCore.xcframework TobogganApp/Frameworks/
   
   # Add Swift bindings
   cp target/uniffi/TobogganCore.swift TobogganApp/TobogganApp/Generated/
   ```

3. **Replace Mock Implementation**
   ```swift
   // Before: Mock types
   struct MockSlide: SlideProtocol { /* ... */ }
   
   // After: Real Rust types via UniFFI
   import TobogganCore
   
   class PresentationViewModel: ObservableObject {
       private let client: TobogganClient
       @Published var state: PresentationState = .disconnected
       
       func connect(to url: String) async {
           try await client.connect(url: url)
       }
   }
   ```

4. **Configure Networking**
   ```swift
   let config = ClientConfig(
       serverUrl: "ws://localhost:8080/api/ws",
       reconnectInterval: Duration.seconds(5),
       commandTimeout: Duration.seconds(10)
   )
   let client = createClient(config: config)
   ```

#### Migration Benefits
- **Shared Logic**: Common codebase with other Toboggan clients
- **Real-time Sync**: WebSocket-based multi-client synchronization
- **Type Safety**: UniFFI ensures type-safe boundaries
- **Performance**: Efficient Rust implementation with zero-copy data
- **Reliability**: Robust error handling and automatic reconnection

## Development Workflow

### Current Development (Swift-Only)
```bash
# Fast UI iteration - no Rust compilation needed
open TobogganApp/TobogganApp.xcodeproj
# Edit Swift files, use SwiftUI previews, test immediately
```

### Future Development (With Rust)
```bash
# 1. Update Rust code
cd toboggan-mobile
# Edit .rs files

# 2. Rebuild framework
mise build:ios

# 3. Update iOS app
cd ../TobogganApp
# Edit Swift code to use new Rust types

# 4. Test integration
open TobogganApp.xcodeproj
```

### Testing Strategy
- **Unit Tests**: Test Rust logic independently
- **Integration Tests**: Test UniFFI boundaries
- **UI Tests**: Test Swift UI with real data
- **Network Tests**: Test WebSocket connectivity

## Troubleshooting

### Build Issues
```bash
# Check iOS targets are installed
rustup target list | grep apple-ios

# Install missing targets
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim

# Clean build if needed
cargo clean
./build.sh
```

### UniFFI Issues
```bash
# Regenerate bindings
cargo run --bin uniffi-bindgen generate src/toboggan.udl --language swift

# Check binding compatibility
uniffi-bindgen --version
```

### Xcode Integration
- Ensure framework is properly linked in Xcode project
- Verify bridging header is configured correctly  
- Check that generated Swift files are in project

### Network Debugging
```swift
// Enable network logging
let config = ClientConfig(
    serverUrl: "ws://localhost:8080/api/ws",
    reconnectInterval: Duration.seconds(1),
    commandTimeout: Duration.seconds(30)
)

// Test connection manually
let client = createClient(config: config)
Task {
    do {
        try await client.connect(url: config.serverUrl)
        print("Connected successfully")
    } catch {
        print("Connection failed: \(error)")
    }
}
```

## Contributing

### Code Organization
- Keep business logic in Rust for cross-platform consistency
- Use Swift only for iOS-specific UI and integration
- Follow UniFFI best practices for boundary design
- Maintain comprehensive test coverage

### Development Guidelines
- Test Rust changes with `cargo test`
- Validate UniFFI bindings generation
- Ensure iOS app builds with new framework
- Test on both simulator and physical devices

The current pure Swift approach enables rapid UI development, while this Rust foundation provides the infrastructure for production server connectivity when ready.