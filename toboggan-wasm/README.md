# Toboggan WASM

A WebAssembly implementation of the Toboggan presentation client using Rust.

## Overview

This crate provides a WASM-based presentation client that connects to the Toboggan server and displays slides with real-time synchronization. It's designed to be lightweight and performant while maintaining the same functionality as the TypeScript implementation.

## Features

- **WebSocket Connection**: Real-time communication with Toboggan server
- **Slide Rendering**: Support for Text, HTML, IFrame, HBox, VBox content types
- **Navigation**: Keyboard and button-based slide navigation
- **Timer Display**: Auto-updating duration display with hh:mm:ss format
- **State Management**: Handles Running, Paused, and Done presentation states
- **Minimal Dependencies**: Uses gloo utilities and minimal web-sys features

## Architecture

The implementation follows these design principles:

### Memory Safety & Management
- **Safe Memory Management**: Uses `Rc<RefCell<T>>` pattern instead of unsafe raw pointers
- **No Unsafe Code**: Completely eliminated unsafe code blocks for better security
- **Automatic Cleanup**: Proper cleanup of event listeners, closures, and resources
- **Optimized Allocations**: Pre-allocated string buffers and reduced allocations in hot paths

### Security
- **HTML Sanitization**: All HTML content is sanitized to prevent XSS attacks
- **URL Validation**: Only safe URLs (HTTPS, localhost) are allowed in iframes
- **Input Escaping**: All user input is properly escaped before DOM insertion
- **Sandboxed Iframes**: Iframes use sandbox attributes for additional security

### Performance
- **String Buffer Optimization**: Uses pre-allocated buffers for HTML generation
- **Minimal Dependencies**: Optimized dependency footprint using gloo instead of full web-sys
- **Aggressive Size Optimization**: Build configuration optimized for minimal WASM size
- **Connection Retry Logic**: Automatic reconnection with exponential backoff

### Dependencies
- **gloo**: Provides clean abstractions for timers, events, and utilities
- **web-sys**: Minimal feature set for DOM manipulation and WebSocket
- **toboggan-core**: Shared types and protocol implementation
- **serde_json**: JSON serialization for WebSocket communication

### Key Components

#### TobogganApp
Main application struct managing:
- WebSocket connection and event handlers
- DOM element references and event listeners
- Timer management for duration display
- Slide cache and rendering

#### Event Management
- Automatic cleanup of event listeners on disposal
- Keyboard shortcuts (arrows, space, home, end, p/r for pause/resume)
- Navigation button click handlers

#### Content Rendering
- HTML generation from toboggan-core Content types
- HTML escaping for security
- Layout support (HBox/VBox)
- Notes display with collapsible details

## Building

### Prerequisites
```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack for easier building (optional)
cargo install wasm-pack
```

### Build Commands
```bash
# From workspace root
cargo build -p toboggan-wasm --target wasm32-unknown-unknown

# With wasm-pack (generates JS bindings)
wasm-pack build --target web --out-dir pkg
```

### Size Optimization
The build is configured for minimum size:
- `opt-level = "z"` - Optimize for size even more aggressively
- `lto = "fat"` - More aggressive Link Time Optimization
- `codegen-units = 1` - Single codegen unit
- `panic = "abort"` - Smaller panic handling
- `strip = true` - Strip symbols for smaller size
- Removed deprecated `wee_alloc` (default allocator is now better)

## Usage

### HTML Integration
```html
<script type="module">
    import init, { init_app, init_app_with_config, TobogganConfig } from './pkg/toboggan_wasm.js';
    
    async function run() {
        await init();
        
        // Option 1: Use default configuration
        const app = init_app();
        app.start();
        
        // Option 2: Use custom configuration
        const config = new TobogganConfig();
        config.set_websocket_url('ws://custom-server:8080/api/ws');
        config.set_auto_retry(true);
        config.set_retry_attempts(5);
        const customApp = init_app_with_config(config);
        customApp.start();
    }
    
    run();
</script>
```

### Required DOM Elements
The application expects these elements in the HTML:
- `#connection-status` - Connection status display
- `#slide-counter` - Current slide position
- `#duration-display` - Presentation duration
- `#error-display` - Error messages
- `#app` - Main slide content area
- Navigation buttons: `#first-btn`, `#prev-btn`, `#next-btn`, `#last-btn`, `#pause-btn`, `#resume-btn`

### Keyboard Shortcuts
- **Arrow Keys/Space**: Navigate slides
- **Home/End**: Jump to first/last slide
- **P**: Pause presentation
- **R**: Resume presentation

## Protocol

Uses the same WebSocket protocol as other Toboggan clients:

### Commands (Client → Server)
- `Register`: Register as HTML renderer client
- `First/Last/Next/Previous`: Navigation commands
- `Pause/Resume`: Presentation control

### Notifications (Server → Client)
- `State`: Current presentation state with slide info
- `Error`: Error messages
- `Pong`: Heartbeat response

## Development

### Code Structure
```
src/
├── lib.rs              # Main implementation
├── content.rs          # Content rendering (shared via toboggan-core)
└── ...
```

### Key Implementation Details
- **Safe Reference Management**: Uses `Rc<RefCell<T>>` pattern for safe WASM callbacks
- **Event Listener Lifecycle**: Automatic cleanup to prevent memory leaks
- **Performance API Integration**: Accurate timing using browser Performance API
- **Error Categorization**: Different error types with appropriate user feedback
- **HTML Security**: Sanitization and validation of all dynamic content
- **Connection Management**: Automatic retry with exponential backoff

## Limitations

- Terminal content (`Content::Term`) not supported in WASM environment
- Requires modern browser with WebAssembly and ES modules support
- Limited to toboggan-core's no_std feature set

## Testing

Run tests with:
```bash
# Unit tests
cargo test -p toboggan-wasm

# WASM tests (requires wasm-pack)
wasm-pack test --headless --firefox toboggan-wasm
```

## Future Improvements

- Implement actual slide fetching from REST API
- Add slide preloading and caching
- Implement slide transitions and animations
- Add touch/gesture support for mobile devices
- Add more comprehensive test coverage
- Implement slide caching strategies

## Recent Improvements (v0.1.0)

✅ **Security Hardening**
- Eliminated all unsafe code using safe `Rc<RefCell<T>>` pattern
- Added HTML sanitization to prevent XSS attacks
- Implemented URL validation for iframe content
- Added sandboxed iframe security attributes

✅ **Performance Optimization**
- Optimized string allocation with pre-allocated buffers
- Reduced memory allocations in rendering hot paths
- Improved build size optimization settings
- Removed deprecated `wee_alloc` dependency

✅ **Error Handling**
- Added comprehensive error types with categorization
- Implemented retry logic with exponential backoff
- Added proper error boundaries with user feedback
- Improved connection state management

✅ **API Design**
- Added configuration options for WebSocket URL and retry behavior
- Implemented proper cleanup lifecycle for memory management
- Added comprehensive test coverage for core functionality
- Improved documentation with security and performance details