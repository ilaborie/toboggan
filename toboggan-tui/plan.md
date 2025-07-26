# Toboggan TUI Client Implementation Plan

## Overview
Create a terminal-based user interface (TUI) for Toboggan presentations using the `ratatui` framework. This client provides a efficient, keyboard-driven presentation experience that works in any terminal environment, perfect for developers and system administrators.

## Architecture

### Core Components

The TUI follows the Model-View-Controller pattern adapted for terminal interfaces:

```
┌─────────────────────────────────────────────────────┐
│                   Terminal UI                       │
├─────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────────────┐    │
│  │   Status Bar    │  │     Help Panel          │    │
│  ├─────────────────┤  └─────────────────────────┘    │
│  │                 │  ┌─────────────────────────┐    │
│  │  Slide Content  │  │    Control Panel        │    │
│  │                 │  │                         │    │
│  │                 │  │ [F] First  [P] Previous │    │
│  │                 │  │ [N] Next   [L] Last     │    │
│  │                 │  │ [Space] Play/Pause      │    │
│  │                 │  │ [Q] Quit   [H] Help     │    │
│  └─────────────────┘  └─────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

1. **Application (app.rs)**
   - Main application state and event loop
   - Coordinate between UI and business logic
   - Handle keyboard input and terminal events

2. **UI Components (ui/)**
   - `mod.rs` - Common UI utilities and layouts
   - `slide_view.rs` - Main slide content display
   - `status_bar.rs` - Connection status and slide info
   - `control_panel.rs` - Navigation shortcuts display
   - `help_panel.rs` - Keyboard shortcuts help
   - `error_popup.rs` - Error message overlay

3. **Services (services/)**
   - `websocket.rs` - Async WebSocket client
   - `slide_client.rs` - HTTP client for slide fetching
   - `content_renderer.rs` - Terminal-friendly content rendering
   - `config.rs` - Configuration management

4. **State Management (state.rs)**
   - Application state with atomic updates
   - Thread-safe communication between async tasks
   - UI state separate from presentation state

### Event-Driven Architecture

```rust
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Terminal events
    Key(KeyEvent),
    Resize(u16, u16),
    Tick,
    
    // WebSocket events
    Connected,
    Disconnected,
    NotificationReceived(Notification),
    ConnectionError(String),
    
    // Application events
    SlideLoaded(SlideId, Slide),
    SlideLoadError(SlideId, String),
    Quit,
}

#[derive(Debug, Clone)]
pub enum AppAction {
    // Navigation commands
    SendCommand(Command),
    
    // UI actions
    ToggleHelp,
    ShowError(String),
    ClearError,
    
    // Application actions
    Connect,
    Disconnect,
    Quit,
}
```

### State Management

```rust
#[derive(Debug)]
pub struct AppState {
    // Connection state
    pub connection_status: ConnectionStatus,
    pub websocket_handle: Option<JoinHandle<()>>,
    
    // Presentation state
    pub current_slide: Option<SlideId>,
    pub slides: HashMap<SlideId, Slide>,
    pub presentation_state: Option<State>,
    
    // UI state
    pub show_help: bool,
    pub error_message: Option<String>,
    pub terminal_size: (u16, u16),
    
    // Configuration
    pub config: TuiConfig,
}
```

## Implementation Steps

### Phase 1: Core Infrastructure
1. **Project Setup**
   - Add ratatui and crossterm dependencies
   - Create basic terminal setup and cleanup
   - Implement event loop with crossterm

2. **Basic UI Layout**
   - Create main layout with panels
   - Implement responsive design for different terminal sizes
   - Add basic keyboard input handling

3. **Configuration System**
   - Config file support (~/.config/toboggan/tui.toml)
   - Command-line argument parsing
   - Environment variable support

### Phase 2: WebSocket Integration
1. **Async WebSocket Client**
   - Implement tokio-tungstenite client
   - Event-driven communication with UI thread
   - Connection retry logic with exponential backoff

2. **State Synchronization**
   - Handle incoming notifications
   - Update UI state based on presentation changes
   - Manage client registration and heartbeat

3. **Command Handling**
   - Send navigation commands via WebSocket
   - Handle command errors and feedback
   - Queue commands during disconnection

### Phase 3: Content Rendering
1. **Slide Content Display**
   - Terminal-friendly text rendering
   - Basic HTML-to-text conversion
   - Markdown rendering with syntax highlighting
   - Responsive text wrapping and scrolling

2. **UI Components**
   - Status bar with connection and slide info
   - Control panel with keyboard shortcuts
   - Help overlay with all commands
   - Error popup with dismissal

### Phase 4: Advanced Features
1. **Enhanced Navigation**
   - Slide thumbnails/preview (ASCII art)
   - Search functionality for slides
   - Bookmarks for quick navigation

2. **Presentation Features**
   - Timer display with duration tracking
   - Notes view (if available)
   - Slide progress indicator

3. **Terminal Optimizations**
   - Mouse support (optional)
   - Color theme customization
   - Terminal detection and optimization

## File Structure

```
toboggan-tui/
├── Cargo.toml
├── plan.md
└── src/
    ├── main.rs              # Entry point and CLI
    ├── app.rs               # Main application logic
    ├── state.rs             # Application state management
    ├── events.rs            # Event types and handling
    ├── config.rs            # Configuration management
    ├── ui/
    │   ├── mod.rs           # UI utilities and themes
    │   ├── layout.rs        # Main layout management
    │   ├── slide_view.rs    # Slide content display
    │   ├── status_bar.rs    # Status and progress bar
    │   ├── control_panel.rs # Navigation controls display
    │   ├── help_panel.rs    # Help overlay
    │   └── error_popup.rs   # Error message popup
    ├── services/
    │   ├── mod.rs
    │   ├── websocket.rs     # WebSocket client
    │   ├── slide_client.rs  # HTTP slide fetching
    │   └── renderer.rs      # Content rendering
    ├── terminal/
    │   ├── mod.rs
    │   ├── setup.rs         # Terminal initialization
    │   └── input.rs         # Keyboard input handling
    └── utils/
        ├── mod.rs
        ├── formatting.rs    # Text formatting utilities
        └── colors.rs        # Color scheme management
```

## Key Features

### Core Functionality
- ✅ Real-time WebSocket synchronization
- ✅ Keyboard-driven navigation
- ✅ Connection status display
- ✅ Error handling with user feedback
- ✅ Configuration via file and CLI args

### Terminal-Specific Features
- ✅ Responsive layout for different terminal sizes
- ✅ Keyboard shortcuts with help overlay
- ✅ Terminal-optimized content rendering
- ✅ Color themes (dark/light/custom)
- ✅ Mouse support (optional)

### Content Rendering
- ✅ Plain text with proper formatting
- ✅ Markdown with syntax highlighting
- ✅ HTML-to-text conversion
- ✅ Unicode support for symbols and formatting

### Navigation Features
- ✅ Standard navigation (First, Previous, Next, Last)
- ✅ Play/Pause presentation control
- ✅ Slide counter and progress display
- ✅ Search functionality
- ✅ Quick help access

## Keyboard Shortcuts

```
Navigation:
  F / Home     - First slide
  P / ←        - Previous slide  
  N / → / Space - Next slide
  L / End      - Last slide
  
Presentation:
  Space        - Play/Pause toggle
  
Application:
  H / ?        - Toggle help panel
  R            - Reconnect WebSocket
  C            - Clear error message
  Q / Ctrl+C   - Quit application
  
Scrolling (long content):
  ↑ / ↓        - Scroll slide content
  PgUp / PgDn  - Page up/down
```

## Configuration File (~/.config/toboggan/tui.toml)

```toml
[connection]
websocket_url = "ws://localhost:8080/api/ws"
api_base_url = "http://localhost:8080"
max_retries = 5
retry_delay_ms = 1000
heartbeat_interval_s = 30

[ui]
theme = "dark"  # "dark", "light", "auto"
show_help_on_start = true
mouse_support = false
unicode_symbols = true

[keybindings]
# Custom key bindings (optional)
quit = "q"
help = "h"
first = "f"
previous = "p"
next = "n"
last = "l"
```

## Dependencies

```toml
[dependencies]
toboggan-core = {path = "../toboggan-core", features = ["std"]}
ratatui = "0.29"           # Terminal UI framework
crossterm = "0.28"         # Cross-platform terminal manipulation
tokio = { workspace = true, features = ["full"] }
tokio-tungstenite = "0.21" # WebSocket client
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter"] }
clap = { workspace = true, features = ["env", "derive"] }
futures = { workspace = true }
reqwest = { version = "0.12", features = ["json"] }
dirs = "5.0"               # Config directory discovery
pulldown-cmark = "0.11"    # Markdown parsing
html2text = "0.12"         # HTML to text conversion
syntect = "5.2"            # Syntax highlighting
```

## Implementation Example

### Main Application Loop
```rust
pub struct TuiApp {
    state: AppState,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    event_rx: Receiver<AppEvent>,
}

impl TuiApp {
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // Start WebSocket client
        self.start_websocket_client().await?;
        
        // Main event loop
        loop {
            // Render UI
            self.terminal.draw(|f| self.render_ui(f))?;
            
            // Handle events
            match self.event_rx.recv().await {
                Some(AppEvent::Key(key)) => {
                    if self.handle_key_event(key).await? {
                        break; // Quit requested
                    }
                }
                Some(AppEvent::NotificationReceived(notification)) => {
                    self.handle_notification(notification).await?;
                }
                Some(AppEvent::Quit) => break,
                None => break,
            }
        }
        
        Ok(())
    }
}
```

### UI Rendering
```rust
fn render_ui(&self, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Control panel
        ])
        .split(f.size());
    
    // Render status bar
    self.render_status_bar(f, chunks[0]);
    
    // Render main content
    if let Some(slide) = &self.current_slide {
        self.render_slide_content(f, chunks[1], slide);
    } else {
        self.render_loading_message(f, chunks[1]);
    }
    
    // Render control panel
    self.render_control_panel(f, chunks[2]);
    
    // Render help overlay if active
    if self.state.show_help {
        self.render_help_overlay(f);
    }
    
    // Render error popup if present
    if let Some(error) = &self.state.error_message {
        self.render_error_popup(f, error);
    }
}
```

## Testing Strategy

1. **Unit Tests**
   - WebSocket client functionality
   - Content rendering logic
   - State management

2. **Integration Tests**
   - End-to-end presentation flow
   - Terminal input/output
   - WebSocket synchronization

3. **Manual Testing**
   - Different terminal emulators
   - Various terminal sizes
   - Keyboard navigation

## Clean Code Standards

- **Single Responsibility**: Each module has a clear, focused purpose
- **Error Handling**: Comprehensive error handling with user-friendly messages
- **Async Safety**: Proper async/await usage with tokio
- **Resource Management**: Proper terminal cleanup and resource disposal
- **Documentation**: Inline documentation for all public APIs
- **Testing**: Unit tests for business logic, integration tests for UI flow

## Performance Considerations

- **Efficient Rendering**: Only redraw when state changes
- **Memory Management**: Efficient slide caching and cleanup
- **Terminal Optimization**: Minimize terminal I/O operations
- **Async Tasks**: Non-blocking WebSocket and HTTP operations

This TUI client provides a powerful, keyboard-driven presentation experience that complements the other Toboggan clients while offering unique advantages for terminal-based workflows.