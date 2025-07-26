# Toboggan iOS Client

A native iOS application for Toboggan presentations built with SwiftUI and Rust.

## Overview

This iOS client provides a native mobile interface for controlling and viewing Toboggan presentations. It uses a hybrid architecture with:

- **SwiftUI**: Native iOS UI and user experience
- **Rust**: Shared business logic and WebSocket communication
- **UniFFI**: Type-safe bridge between Swift and Rust

## Features

- ✅ Real-time WebSocket connection to Toboggan server
- ✅ Native SwiftUI interface optimized for iOS
- ✅ Slide navigation controls (Next, Previous, First, Last)
- ✅ Play/Pause presentation controls
- ✅ Connection status and presentation state indicators
- ✅ Support for different slide types (Cover, Part, Standard)
- ✅ Speaker notes display
- ✅ Responsive design for iPhone and iPad
- ✅ Dark mode support

## Architecture

### Hybrid Architecture

```
SwiftUI Views → ViewModels → Rust Core → WebSocket Server
     ↑                                          ↓
UI Updates ←    ← State Changes ←    ← Notifications
```

### Project Structure

```
TobogganApp/
├── App/
│   ├── TobogganApp.swift         # App entry point
│   └── ContentView.swift         # Main content view with connection logic
├── ViewModels/
│   └── PresentationViewModel.swift # ObservableObject for state management
├── Views/
│   ├── PresentationView.swift    # Main presentation interface
│   ├── SlideView.swift           # Individual slide rendering
│   ├── ControlsView.swift        # Navigation and playback controls
│   └── StatusBarView.swift       # Connection and state status
├── Services/
│   └── (Future: additional services)
├── Utils/
│   ├── MockTypes.swift           # Development mock types
│   ├── Constants.swift           # App constants
│   └── Extensions.swift          # Utility extensions
└── Resources/
    └── Info.plist               # App configuration
```

## Development Setup

### Prerequisites

- Xcode 15.0+
- iOS 16.0+ deployment target
- Rust toolchain with iOS targets

### Building the Rust Core

1. Install iOS targets:
```bash
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim
```

2. Build the Rust library and generate Swift bindings:
```bash
./build.sh
```

This will:
- Build the Rust library for all iOS targets
- Generate Swift bindings using UniFFI
- Create an XCFramework for Xcode integration

### Xcode Project Setup

1. Create a new iOS project in Xcode
2. Add the generated Swift Package as a dependency
3. Link the generated XCFramework
4. Copy the Swift source files to your Xcode project
5. Configure build settings and Info.plist

### Development Workflow

1. **Mock Development**: The app includes mock types for development without the full Rust build
2. **Rust Integration**: Replace mock types with generated UniFFI bindings
3. **Testing**: Use iOS Simulator and physical devices for testing
4. **Server Connection**: Ensure Toboggan server is running and accessible

## Configuration

### Default Settings

- **Server URL**: `ws://localhost:3000/api/ws`
- **Max Retries**: 5
- **Retry Delay**: 1000ms

### Customization

Update `AppConstants.swift` to modify default values:

```swift
struct AppConstants {
    static let defaultServerURL = "ws://your-server:3000/api/ws"
    static let maxRetries: UInt32 = 3
    static let retryDelayMs: UInt64 = 2000
}
```

## Usage

### Connecting to Server

1. Launch the app
2. Enter your Toboggan server WebSocket URL
3. Tap "Connect" to establish connection
4. The app will load presentation data automatically

### Presentation Controls

- **Navigation**: Use Previous/Next buttons or swipe gestures
- **Jump**: First/Last buttons for quick navigation
- **Playback**: Play/Pause button for presentation timing
- **Status**: View connection status and slide progress

### Slide Types

- **Cover**: Title slides with special styling
- **Part**: Section dividers with distinctive appearance  
- **Standard**: Regular content slides

## Technical Details

### State Management

The `PresentationViewModel` manages:
- Connection state and error handling
- Current slide and presentation data
- WebSocket communication with server
- UI state updates via `@Published` properties

### UniFFI Integration

Generated Swift types from Rust:
- `TobogganClient`: Main client interface
- `ClientConfig`: Connection configuration
- `Slide`, `TalkInfo`: Presentation data types
- `Command`, `State`: Control and state types
- `TobogganError`: Error handling

### Performance Considerations

- Async/await for all network operations
- Lazy loading of slide content
- Efficient SwiftUI view updates
- Memory-safe Rust core with no leaks

## Testing

### Development Testing
- Use mock types for UI development
- Test all connection states and error scenarios
- Verify responsive design on different screen sizes

### Integration Testing
- Test with actual Toboggan server
- Verify WebSocket communication
- Test multi-client synchronization

### Device Testing
- iPhone (portrait/landscape)
- iPad (split view, multitasking)
- Different iOS versions

## Troubleshooting

### Common Issues

1. **Build Failures**: Ensure Rust targets are installed and build.sh runs successfully
2. **Connection Issues**: Verify server URL and network connectivity
3. **Missing Bindings**: Re-run build.sh to regenerate Swift bindings
4. **Xcode Integration**: Check XCFramework linking and Swift Package setup

### Debug Mode

Enable debug logging by setting build configuration:
```swift
#if DEBUG
// Debug-specific code
#endif
```

### Performance Monitoring

Monitor key metrics:
- Connection establishment time
- Slide loading performance  
- Memory usage during long presentations
- Battery usage during active sessions

## Future Enhancements

- [ ] Offline presentation mode
- [ ] Slide thumbnails and overview
- [ ] Presentation recording
- [ ] Apple Watch companion app
- [ ] AirPlay support for external displays
- [ ] Voice control integration
- [ ] Advanced gesture controls

## Contributing

1. Follow Swift and SwiftUI best practices
2. Maintain separation between UI and business logic
3. Add appropriate error handling
4. Include unit tests for new functionality
5. Update documentation for API changes

## License

This project is part of the Toboggan presentation system.