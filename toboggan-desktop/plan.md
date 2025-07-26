# Toboggan Desktop Client Implementation Plan

## Overview
Create a cross-platform desktop application for Toboggan presentations using the `iced` GUI framework. The application will provide a native desktop experience with real-time synchronization capabilities.

## Architecture

### Core Components

1. **Main Application (main.rs)**
   - Entry point and application lifecycle management
   - Window configuration and initialization
   - Error handling and recovery

2. **Presentation Model (presentation.rs)**
   - State management for the presentation
   - WebSocket connection handling
   - Slide data caching and management

3. **UI Components (ui/)**
   - `mod.rs` - Common UI utilities and themes
   - `presentation_view.rs` - Main presentation display
   - `controls.rs` - Navigation controls and buttons
   - `status_bar.rs` - Connection status and slide counter
   - `error_display.rs` - Error message overlay

4. **Services (services/)**
   - `websocket.rs` - WebSocket client implementation
   - `slide_renderer.rs` - Content rendering engine
   - `config.rs` - Application configuration

### Message System

Using iced's message-driven architecture:

```rust
#[derive(Debug, Clone)]
pub enum Message {
    // WebSocket events
    WebSocketConnected,
    WebSocketDisconnected,
    WebSocketError(String),
    NotificationReceived(Notification),
    
    // Navigation commands
    FirstSlide,
    PreviousSlide,
    NextSlide,
    LastSlide,
    PlayPresentation,
    PausePresentation,
    
    // UI events
    WindowResized,
    ConfigUpdated(Config),
    Tick, // For timer updates
}
```

### State Management

```rust
#[derive(Debug, Clone)]
pub struct TobogganApp {
    // Connection state
    connection_status: ConnectionStatus,
    websocket: Option<WebSocketHandle>,
    
    // Presentation state
    current_slide: Option<SlideId>,
    slides: HashMap<SlideId, Slide>,
    presentation_state: State,
    
    // UI state
    error_message: Option<String>,
    config: Config,
    
    // Services
    slide_renderer: SlideRenderer,
}
```

## Implementation Steps

### Phase 1: Core Infrastructure
1. **Project Setup**
   - Add iced dependencies to Cargo.toml
   - Create basic application structure
   - Set up workspace integration

2. **Basic Window and UI**
   - Create main window with iced
   - Implement basic layout (header, content, controls)
   - Add placeholder components

3. **Configuration System**
   - Create config struct for WebSocket URL, retry settings
   - Implement config file loading/saving
   - Add command-line argument parsing

### Phase 2: Presentation Core
1. **WebSocket Integration**
   - Implement async WebSocket client using tokio-tungstenite
   - Integrate with iced's async runtime
   - Add connection retry logic with exponential backoff

2. **Slide Management**
   - Implement slide fetching from server API
   - Create slide caching system
   - Add slide content rendering

3. **State Synchronization**
   - Handle state notifications from server
   - Update UI based on presentation state
   - Implement timer for duration tracking

### Phase 3: UI Implementation
1. **Navigation Controls**
   - Create button widgets for navigation
   - Implement keyboard shortcut handling
   - Add visual feedback for button states

2. **Slide Display**
   - Implement content rendering (Text, HTML, Markdown)
   - Add proper typography and styling
   - Support for responsive layout

3. **Status and Error Handling**
   - Connection status indicator
   - Slide counter and progress
   - Error message overlay system

### Phase 4: Polish and Features
1. **Theming**
   - Implement dark/light theme support
   - Consistent color scheme and typography
   - Accessibility considerations

2. **Performance Optimization**
   - Slide preloading
   - Efficient rendering
   - Memory management

3. **Additional Features**
   - Fullscreen presentation mode
   - Multi-monitor support
   - Presentation notes view

## Dependencies

```toml
[dependencies]
toboggan-core = { path = "../toboggan-core", features = ["std"] }
iced = { version = "0.12", features = ["tokio", "advanced"] }
tokio = { workspace = true, features = ["full"] }
tokio-tungstenite = "0.21"
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
clap = { workspace = true, features = ["derive"] }
dirs = "5.0" # For config file location
reqwest = { version = "0.12", features = ["json"] } # For slide fetching
```

## File Structure

```
toboggan-desktop/
├── Cargo.toml
├── plan.md
└── src/
    ├── main.rs              # Entry point
    ├── app.rs               # Main application struct
    ├── config.rs            # Configuration management
    ├── messages.rs          # Message definitions
    ├── ui/
    │   ├── mod.rs           # UI utilities and theme
    │   ├── presentation_view.rs  # Main slide display
    │   ├── controls.rs      # Navigation controls
    │   ├── status_bar.rs    # Status and counter
    │   └── error_display.rs # Error overlay
    ├── services/
    │   ├── mod.rs
    │   ├── websocket.rs     # WebSocket client
    │   ├── slide_client.rs  # HTTP slide fetching
    │   └── renderer.rs      # Content rendering
    └── utils/
        ├── mod.rs
        ├── keyboard.rs      # Keyboard handling
        └── async_utils.rs   # Async helpers
```

## Key Features

### Core Functionality
- ✅ Real-time WebSocket synchronization
- ✅ Navigation controls (First, Previous, Next, Last)
- ✅ Play/Pause presentation control
- ✅ Keyboard shortcuts (Arrow keys, Space, Home, End)
- ✅ Connection status display
- ✅ Error handling and retry logic

### Desktop-Specific Features
- ✅ Native window management
- ✅ System tray integration (optional)
- ✅ Fullscreen presentation mode
- ✅ Multi-monitor support
- ✅ Native file dialogs for configuration

### Content Rendering
- ✅ Text content with proper typography
- ✅ HTML content rendering
- ✅ Markdown content rendering
- ✅ Responsive layout for different window sizes

## Testing Strategy

1. **Unit Tests**
   - WebSocket client functionality
   - Configuration management
   - Message handling logic

2. **Integration Tests**
   - End-to-end presentation flow
   - WebSocket reconnection scenarios
   - Slide loading and caching

3. **Manual Testing**
   - Cross-platform compatibility (Windows, macOS, Linux)
   - Keyboard navigation
   - Multi-client synchronization

## Clean Code Standards

- **Modular Design**: Each component has a single responsibility
- **Error Handling**: Comprehensive error handling with proper user feedback
- **Type Safety**: Leverage Rust's type system for correctness
- **Documentation**: Comprehensive inline documentation
- **Testing**: Unit tests for all business logic
- **Performance**: Efficient rendering and minimal resource usage

This plan ensures a robust, native desktop client that integrates seamlessly with the Toboggan ecosystem while providing an excellent user experience.