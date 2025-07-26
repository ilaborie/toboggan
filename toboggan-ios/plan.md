# Toboggan iOS Client Implementation Plan

## Overview
Create a native iOS application for Toboggan presentations using SwiftUI for the UI and Rust for the core logic. This hybrid approach leverages SwiftUI's native iOS experience while maintaining shared business logic with other Toboggan clients through Rust.

## Architecture

### Hybrid Architecture: SwiftUI + Rust Core

The application uses **UniFFI** to generate Swift bindings for Rust code, enabling:
- **SwiftUI**: Native iOS UI, animations, and platform integration
- **Rust Core**: Shared business logic, WebSocket handling, and data management
- **UniFFI Bridge**: Type-safe communication between Swift and Rust

### Core Components

#### Rust Core (`src/lib.rs`)
```rust
// Shared with other clients via toboggan-core
pub struct TobogganClient {
    websocket: WebSocketClient,
    state: PresentationState,
    config: ClientConfig,
}

// UniFFI exposed interface
#[uniffi::export]
impl TobogganClient {
    #[uniffi::constructor]
    pub fn new(config: ClientConfig) -> Arc<Self>;
    
    pub async fn connect(&self) -> Result<(), TobogganError>;
    pub async fn send_command(&self, command: Command) -> Result<(), TobogganError>;
    pub fn get_current_state(&self) -> Option<State>;
    pub fn get_slide(&self, slide_id: SlideId) -> Option<Slide>;
}
```

#### Swift UI Layer
- **ContentView.swift**: Main presentation view
- **PresentationViewModel.swift**: ObservableObject for state management
- **ControlsView.swift**: Navigation controls
- **SlideView.swift**: Individual slide rendering
- **StatusBar.swift**: Connection status and progress

### Message Flow

```
SwiftUI View → ViewModel → Rust Core → WebSocket Server
                 ↑                           ↓
     UI Updates ←  ← Callbacks ← Notifications
```

## Implementation Steps

### Phase 1: Rust Core Setup
1. **UniFFI Integration**
   - Add UniFFI dependencies and configuration
   - Create UDL (UniFFI Definition Language) file
   - Set up build system for Swift binding generation

2. **Core Rust Library**
   - Implement TobogganClient with UniFFI exports
   - WebSocket client with async callback system
   - State management with thread-safe access
   - Error handling with Swift-compatible types

3. **Swift Bindings Generation**
   - Configure build.rs for UniFFI codegen
   - Generate Swift package with bindings
   - Set up Xcode integration

### Phase 2: iOS Project Setup
1. **Xcode Project Creation**
   - Create new iOS SwiftUI project
   - Configure Swift Package Manager for Rust bindings
   - Set up proper build phases and dependencies

2. **ViewModel Architecture**
   - Create ObservableObject for state management
   - Implement Swift async/await bridge to Rust
   - Handle Rust callback conversion to Combine publishers

3. **Basic UI Structure**
   - Main presentation view layout
   - Navigation structure
   - Status bar and controls placeholder

### Phase 3: Core Functionality
1. **WebSocket Integration**
   - Implement async WebSocket client in Rust
   - Create callback system for Swift integration
   - Add connection retry logic with exponential backoff

2. **Presentation State Management**
   - State synchronization between Rust and Swift
   - Slide data caching and management
   - Timer integration for duration tracking

3. **Content Rendering**
   - Text content rendering with SwiftUI
   - HTML content using WKWebView
   - Markdown rendering with native Swift libraries

### Phase 4: iOS-Specific Features
1. **Native iOS Integration**
   - Background/foreground state handling
   - iOS 16+ Lock Screen controls
   - Notification support for state changes

2. **Accessibility**
   - VoiceOver support
   - Dynamic Type support
   - Accessibility identifiers

3. **Polish and Optimization**
   - Dark mode support
   - iPad-specific layouts
   - Performance optimization

## Project Structure

```
toboggan-ios/
├── Cargo.toml              # Rust dependencies
├── build.rs                # UniFFI build script
├── toboggan.udl            # UniFFI interface definition
├── plan.md
├── src/
│   ├── lib.rs              # Main Rust library
│   ├── client.rs           # TobogganClient implementation
│   ├── websocket.rs        # WebSocket client
│   ├── state.rs            # State management
│   ├── callbacks.rs        # Swift callback system
│   └── error.rs            # Error types
├── TobogganApp/            # iOS Xcode project
│   ├── TobogganApp.xcodeproj
│   ├── App/
│   │   ├── TobogganApp.swift        # App entry point
│   │   ├── ContentView.swift        # Main view
│   │   └── TobogganApp-Bridging-Header.h
│   ├── ViewModels/
│   │   ├── PresentationViewModel.swift
│   │   └── ConnectionManager.swift
│   ├── Views/
│   │   ├── PresentationView.swift   # Main slide display
│   │   ├── ControlsView.swift       # Navigation controls
│   │   ├── SlideView.swift          # Individual slide
│   │   ├── StatusBarView.swift      # Status and progress
│   │   └── ErrorView.swift          # Error display
│   ├── Services/
│   │   ├── SlideRenderer.swift      # Content rendering
│   │   └── ConfigManager.swift      # App configuration
│   ├── Utils/
│   │   ├── Extensions.swift         # Utility extensions
│   │   └── Constants.swift          # App constants
│   └── Resources/
│       ├── Assets.xcassets
│       └── Info.plist
└── Package.swift           # Swift Package for Rust bindings
```

## UniFFI Interface Definition (toboggan.udl)

```idl
namespace toboggan {
    [Throws=TobogganError]
    TobogganClient create_client(ClientConfig config);
};

[Error]
enum TobogganError {
    "ConnectionError",
    "ParseError",  
    "ConfigError",
    "UnknownError",
};

dictionary ClientConfig {
    string websocket_url;
    u32 max_retries;
    u64 retry_delay_ms;
};

dictionary Slide {
    SlideId id;
    string title;
    string body;
    SlideKind kind;
};

enum SlideKind {
    "Cover",
    "Part", 
    "Standard",
};

enum Command {
    "Next",
    "Previous",
    "First",
    "Last",
    "Play",
    "Pause",
};

[Enum]
interface State {
    Running(SlideId current, u64 total_duration_ms);
    Paused(SlideId current, u64 total_duration_ms);
    Done(SlideId current, u64 total_duration_ms);
};

callback interface StateCallback {
    void on_state_changed(State state);
    void on_connection_changed(boolean connected);
    void on_error(string message);
};

interface TobogganClient {
    constructor(ClientConfig config);
    
    [Async, Throws=TobogganError]
    void connect();
    
    [Async, Throws=TobogganError] 
    void disconnect();
    
    [Throws=TobogganError]
    void send_command(Command command);
    
    State? get_current_state();
    Slide? get_slide(SlideId slide_id);
    
    void set_state_callback(StateCallback callback);
};
```

## Swift ViewModel Example

```swift
@MainActor
class PresentationViewModel: ObservableObject {
    @Published var currentSlide: Slide?
    @Published var connectionStatus: ConnectionStatus = .disconnected
    @Published var errorMessage: String?
    @Published var presentationState: State?
    
    private var tobogganClient: TobogganClient?
    private var stateCallback: StateCallbackImpl?
    
    func connect(to url: String) async {
        do {
            let config = ClientConfig(
                websocketUrl: url,
                maxRetries: 5,
                retryDelayMs: 1000
            )
            
            tobogganClient = try createClient(config: config)
            stateCallback = StateCallbackImpl(viewModel: self)
            tobogganClient?.setStateCallback(callback: stateCallback!)
            
            try await tobogganClient?.connect()
            connectionStatus = .connected
        } catch {
            errorMessage = "Failed to connect: \(error)"
            connectionStatus = .disconnected
        }
    }
    
    func sendCommand(_ command: Command) {
        do {
            try tobogganClient?.sendCommand(command: command)
        } catch {
            errorMessage = "Failed to send command: \(error)"
        }
    }
}

class StateCallbackImpl: StateCallback {
    weak var viewModel: PresentationViewModel?
    
    init(viewModel: PresentationViewModel) {
        self.viewModel = viewModel
    }
    
    func onStateChanged(state: State) {
        Task { @MainActor in
            viewModel?.presentationState = state
            // Update current slide based on state
        }
    }
    
    func onConnectionChanged(connected: Bool) {
        Task { @MainActor in
            viewModel?.connectionStatus = connected ? .connected : .disconnected
        }
    }
    
    func onError(message: String) {
        Task { @MainActor in
            viewModel?.errorMessage = message
        }
    }
}
```

## Dependencies

### Rust (Cargo.toml)
```toml
[dependencies]
toboggan-core = { path = "../toboggan-core", features = ["std"] }
uniffi = "0.27"
tokio = { workspace = true, features = ["full"] }
tokio-tungstenite = "0.21"
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }

[build-dependencies]
uniffi = { version = "0.27", features = ["build"] }

[lib]
crate-type = ["cdylib", "staticlib"]
name = "toboggan_ios_core"

[[bin]]
name = "uniffi-bindgen"
path = "uniffi-bindgen.rs"
```

### Swift (Package.swift)
```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "TobogganCore",
    platforms: [.iOS(.v16)],
    products: [
        .library(name: "TobogganCore", targets: ["TobogganCore"])
    ],
    targets: [
        .binaryTarget(
            name: "TobogganCore",
            path: "./target/TobogganCore.xcframework"
        )
    ]
)
```

## Key Features

### Core Functionality
- ✅ Real-time WebSocket synchronization
- ✅ Native SwiftUI interface
- ✅ Shared Rust business logic
- ✅ Type-safe Rust ↔ Swift communication
- ✅ Async/await support in both languages

### iOS-Specific Features
- ✅ Background/foreground handling
- ✅ iOS 16+ Lock Screen controls
- ✅ Native navigation and gestures
- ✅ Dark mode support
- ✅ Accessibility (VoiceOver, Dynamic Type)
- ✅ iPad optimized layouts

### Content Rendering
- ✅ Native SwiftUI text rendering
- ✅ HTML content via WKWebView
- ✅ Markdown rendering
- ✅ Responsive design for different screen sizes

## Testing Strategy

1. **Rust Core Tests**
   - Unit tests for business logic
   - WebSocket client functionality
   - UniFFI binding validation

2. **iOS UI Tests**
   - SwiftUI component testing
   - Integration with Rust core
   - Accessibility testing

3. **Manual Testing**
   - iOS device testing (iPhone/iPad)
   - Background/foreground behavior
   - Multi-client synchronization

## Clean Code Standards

- **Separation of Concerns**: Clear boundary between UI (Swift) and logic (Rust)
- **Type Safety**: Leverage both Rust and Swift type systems
- **Error Handling**: Proper error propagation across language boundaries
- **Memory Safety**: Rust's memory safety + Swift's ARC
- **Testing**: Comprehensive testing in both languages
- **Documentation**: Clear API documentation for UniFFI interface

This plan creates a truly native iOS experience while leveraging shared Rust logic, ensuring consistency across all Toboggan clients while providing platform-specific optimizations.