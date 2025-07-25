# Toboggan Web Presentation Application - Implementation Plan

## Overview
Build a minimal web application that connects to the toboggan-server WebSocket endpoint to display presentation slides and provide navigation controls. The application will be rewritten in Rust later, so focus on simplicity and clarity.

## Architecture

### 1. HTML Structure
- Clean, semantic HTML5 structure
- `#app` element as the main container for slide content
- Header with navigation buttons using emoji for visual cues
- Accessible design with proper ARIA labels and keyboard navigation

### 2. WebSocket Communication
- Connect to `ws://localhost:3000/ws` (default toboggan-server endpoint)
- Handle incoming Notifications (state updates, errors)
- Send Commands for navigation (First, Last, Next, Previous, etc.)

### 3. Core Components

#### WebSocket Client
- Auto-reconnection on connection loss
- Message parsing and error handling
- Client registration with Html renderer type

#### Slide Display
- Render current slide content based on Content type:
  - Text: Plain text display
  - Html: Raw HTML insertion
  - Md: Basic markdown rendering (or display as text for simplicity)
  - Empty: Show placeholder

#### Navigation Controls
- 🏠 First slide
- ⬅️ Previous slide  
- ➡️ Next slide
- 🏁 Last slide
- ⏸️ Pause presentation
- ▶️ Resume presentation

## Implementation Strategy

### Phase 1: Clean Existing Code
- Remove TypeScript dependencies and build tools
- Keep only essential HTML structure
- Remove all styling (CSS) for minimal approach

### Phase 2: Basic HTML Structure
- Create semantic HTML with navigation header
- Add #app element for slide content
- Implement accessible button controls

### Phase 3: WebSocket Integration
- Establish WebSocket connection
- Handle connection lifecycle (open, close, error)
- Implement message parsing for Notifications
- Implement command sending functionality

### Phase 4: Slide Rendering
- Parse and display slide content
- Handle different Content types appropriately
- Show slide metadata (current/total slides)

### Phase 5: Navigation Logic
- Wire navigation buttons to send Commands
- Update UI based on current state
- Handle edge cases (first/last slide navigation)

### Phase 6: Error Handling & Polish
- Display connection status
- Show error messages from server
- Implement retry logic for failed connections

## Technical Constraints

### Libraries
- NO external libraries or frameworks
- Use only standard Web APIs (WebSocket, DOM, JSON)
- Vanilla JavaScript for all functionality

### Styling
- NO CSS styling initially
- Focus on raw HTML functionality
- Rely on browser default styles

### Browser Support
- Modern browsers with WebSocket support
- ES6+ JavaScript features allowed
- No polyfills needed

## Data Structures

### WebSocket Messages

#### Outgoing Commands
```json
{ "command": "First" }
{ "command": "Last" }
{ "command": "Next" }
{ "command": "Previous" }
{ "command": "Register", "client": "uuid", "renderer": "Html" }
```

#### Incoming Notifications
```json
{
  "type": "State",
  "timestamp": "2023-07-20T10:30:00Z",
  "state": {
    "Running": {
      "since": "2023-07-20T10:25:00Z",
      "current": 5,
      "total_duration": { "secs": 300, "nanos": 0 }
    }
  }
}
```

## File Structure
```
toboggan-web/
├── plan.md           # This implementation plan
├── devlog.md         # Development progress log
├── index.html        # Main HTML file
└── app.js           # Core JavaScript functionality
```

## Success Criteria
1. ✅ WebSocket connection established with toboggan-server
2. ✅ Receive and display current slide content
3. ✅ Navigation buttons send correct Commands
4. ✅ UI updates reflect server state changes
5. ✅ Application works with riir.toml presentation
6. ✅ Accessible HTML structure with proper semantics
7. ✅ Graceful error handling and reconnection

## Future Considerations
- This is a prototype for Rust rewrite
- Keep code simple and well-documented
- Focus on functionality over aesthetics
- Ensure patterns can be easily translated to Rust/WASM