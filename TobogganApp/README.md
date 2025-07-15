# TobogganApp

Native iOS client for Toboggan presentations built with SwiftUI, providing a professional presentation experience on iOS devices.

## Features

### Presentation Experience
- **Native SwiftUI Interface**: Modern iOS design with system integration
- **Presenter View**: Dedicated view showing current slide, notes, and next slide preview
- **Real-time Synchronization**: WebSocket-based multi-client synchronization
- **Gesture Controls**: Swipe navigation and tap-to-advance
- **External Display Support**: AirPlay and wired external display compatibility

### Content Support
- **Rich Content Rendering**: Full HTML and Markdown slide rendering
- **Responsive Layout**: Adapts to different device orientations and sizes
- **Accessibility**: VoiceOver support for all interface elements
- **Dark Mode**: Full support for system dark mode preferences

## Architecture

TobogganApp follows modern iOS architecture patterns for maintainability and performance:

### Design Patterns
- **MVVM (Model-View-ViewModel)**: Clear separation between UI and business logic
- **SwiftUI**: Declarative UI with reactive data binding
- **Combine Framework**: Reactive programming for state management
- **Coordinator Pattern**: Navigation flow management

### Core Components
- **SwiftUI Views**: Native iOS UI components with system styling
- **ViewModels**: Business logic and state management
- **Services**: Network communication and data persistence
- **Mock Types**: Development-time mocks for rapid UI iteration

### Development Modes
1. **Mock Mode** (Current): Swift-only implementation with mock data
   - Fast compilation and iteration
   - SwiftUI previews without external dependencies
   - Ideal for UI development and testing

2. **Production Mode** (Future): Integration with Rust core via UniFFI
   - Real WebSocket connectivity to Toboggan server
   - Shared business logic with other Toboggan clients
   - Full presentation synchronization

## Getting Started

### Prerequisites
- **Xcode 15.0+** - Latest stable version recommended
- **iOS 16.0+** - Minimum deployment target
- **macOS 13.0+** - For Xcode and development tools

### Building the App

#### Option 1: Using Mise (Recommended)
```bash
# From the workspace root
mise build:ios
```

#### Option 2: Manual Build
```bash
# Navigate to iOS Rust library directory
cd toboggan-ios

# Build the iOS framework (when needed for production mode)
./build.sh

# Return to workspace root
cd ..
```

#### Option 3: iOS-Only Development
```bash
# For UI-only development, no Rust build required
open TobogganApp/TobogganApp.xcodeproj
```

### Running the App

1. Open `TobogganApp/TobogganApp.xcodeproj` in Xcode
2. Select your target device or simulator
3. Press `Cmd+R` to build and run

## Development Workflow

### Rapid UI Development

The app uses mock types for fast development cycles:

```swift
// Mock types enable SwiftUI previews
struct MockSlide: SlideProtocol {
    let id = UUID()
    let title = "Sample Slide"
    let content = "Mock content for development"
}
```

**Benefits:**
- Fast build times (no Rust compilation)
- SwiftUI previews work instantly
- Easy UI iteration and testing
- No external server dependencies

### Project Structure

```
TobogganApp/
├── TobogganApp.xcodeproj/           # Xcode project
├── TobogganApp/
│   ├── TobogganApp.swift           # App entry point
│   ├── Views/                      # SwiftUI views
│   │   ├── ContentView.swift       # Main container view
│   │   ├── SlideView.swift         # Individual slide display
│   │   └── PresenterView.swift     # Presenter mode interface
│   ├── ViewModels/                 # Business logic layer
│   │   ├── PresentationViewModel.swift
│   │   └── SlideViewModel.swift
│   ├── Models/                     # Data types and protocols
│   │   ├── SlideProtocol.swift
│   │   └── PresentationProtocol.swift
│   ├── Services/                   # External integrations
│   │   └── WebSocketService.swift
│   ├── Utils/                      # Utilities and helpers
│   │   ├── MockTypes.swift         # Development mocks
│   │   └── Extensions.swift        # Swift extensions
│   └── Resources/                  # Assets and localizations
├── Tests/                          # Unit and UI tests
└── README.md                       # This file
```

## Testing

### Unit Tests
```bash
# Run unit tests in Xcode
Cmd+U

# Or from command line
xcodebuild test -scheme TobogganApp -destination 'platform=iOS Simulator,name=iPhone 15'
```

### UI Tests
The app includes UI tests for critical presentation flows:
- Slide navigation
- Presenter view transitions
- External display handling

## Future Integration with Rust Core

When ready to integrate with the real Toboggan server:

### Prerequisites for Production Mode
- Rust toolchain with iOS targets installed
- UniFFI-generated Swift bindings
- Built `toboggan-ios` framework

### Integration Steps
1. **Build Rust Framework**: Run `mise build:ios` to generate iOS bindings
2. **Replace Mock Types**: Swap `MockTypes.swift` with real UniFFI-generated types
3. **Update Services**: Connect `WebSocketService` to real Toboggan server
4. **Test Integration**: Verify real-time synchronization works
5. **Update UI**: Adapt views to handle real data and error states

### Benefits of Rust Integration
- Shared business logic with other Toboggan clients
- Real WebSocket connectivity and synchronization
- Consistent presentation behavior across platforms
- Type-safe communication with server

## Contributing to iOS Development

### Code Style Guidelines
- Follow Swift API Design Guidelines
- Use SwiftLint for code consistency
- Maintain SwiftUI best practices
- Document complex business logic

### Common Development Tasks
- **Adding new views**: Create in `Views/` directory with associated ViewModel
- **Updating mock data**: Modify `MockTypes.swift` for development
- **Testing UI changes**: Use SwiftUI previews for rapid iteration
- **Adding features**: Follow MVVM pattern with proper separation

### Performance Considerations
- Use `@StateObject` and `@ObservedObject` appropriately
- Minimize view re-rendering with proper state management
- Optimize image and content loading for smooth scrolling
- Test on physical devices for real performance

## Troubleshooting

### Common Issues

**Build Errors:**
- Ensure Xcode is up to date (15.0+)
- Clean build folder: Product → Clean Build Folder
- Reset simulators if needed

**Preview Issues:**
- Restart Xcode if SwiftUI previews stop working
- Check that mock types conform to required protocols
- Verify preview data is properly initialized

**Runtime Issues:**
- Check console logs in Xcode for error messages
- Verify mock data matches expected formats
- Test on different device sizes and orientations

## License

Part of the Toboggan project. Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../LICENSE-MIT))

at your option.