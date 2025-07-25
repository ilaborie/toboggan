# Toboggan Web Development Log

## 2025-07-20 - Project Initialization

### Goals
Create a minimal web presentation application that:
- Connects to toboggan-server WebSocket
- Displays current slide content 
- Provides navigation controls
- Uses only standard Web APIs (no external libraries)
- Focuses on functionality over styling
- Serves as prototype for future Rust rewrite

### Phase 1: Planning & Research ✅
- [x] Analyzed toboggan-server WebSocket API structure
- [x] Researched Command/Notification message formats
- [x] Created comprehensive implementation plan
- [x] Documented WebSocket protocol details

**Key Findings:**
- WebSocket endpoint: `ws://localhost:3000/ws`
- Commands use tagged JSON: `{"command": "First"}` 
- Notifications include state, error, and pong types
- SlideId is simple u8, ClientId is UUID-based
- Server sends initial state on connection

### Phase 2: Code Cleanup (In Progress)
**Current Task:** Remove TypeScript/Vite setup and create minimal HTML structure

**Next Steps:**
1. Clean existing TypeScript files
2. Create minimal index.html with semantic structure
3. Implement WebSocket connection logic
4. Add slide rendering functionality
5. Wire up navigation controls
6. Test with riir.toml presentation

### Notes
- Keep implementation simple for easy Rust translation
- Focus on accessibility with proper ARIA labels
- Use emoji in navigation buttons for visual clarity
- No CSS styling - rely on browser defaults

---

## Development Sessions

### Session 1: 2025-07-20
**Duration:** Started - Complete
**Focus:** Full implementation from planning to working application

**Completed:**
- ✅ Project analysis and planning
- ✅ WebSocket protocol research (Commands/Notifications)
- ✅ Documentation creation (plan.md, devlog.md)
- ✅ Cleaned TypeScript/Vite setup
- ✅ Implemented minimal, accessible HTML structure
- ✅ Built complete WebSocket client with auto-reconnection
- ✅ Implemented slide rendering for Text, Html, Md, Empty content types
- ✅ Added navigation controls with emoji buttons
- ✅ Implemented keyboard shortcuts (arrows, space, home, end, p/r)
- ✅ Fixed API integration (slides served as object, not individual endpoints)
- ✅ Added proper error handling and status display
- ✅ Successfully tested with riir.toml presentation

**Key Findings:**
- Server runs on port 8080 (not 3000 as shown in OpenAPI docs)
- WebSocket endpoint: `ws://localhost:8080/api/ws`
- Slides API: `http://localhost:8080/api/slides` returns all slides as object
- SlideId starts from 0, displayed as slide number (SlideId + 1)

**Architecture Implemented:**
```
index.html     # Semantic HTML with accessibility features
app.js         # Vanilla JS WebSocket client with full functionality
```

**Features Working:**
- Real-time WebSocket communication
- Slide navigation (first, last, next, previous)
- Presentation control (pause/resume)
- HTML slide content rendering
- Connection status and retry logic
- Keyboard navigation support
- Slide counter with total slides display

**Testing Results:**
- ✅ Server starts successfully with riir.toml
- ✅ API endpoints respond correctly
- ✅ WebSocket connection can be established
- ✅ Navigation commands work through WebSocket
- ✅ Slide content renders properly (tested HTML content from riir.toml)

**Ready for production use as minimal presentation client.**

---

### Session 2: 2025-07-20 - TypeScript Conversion
**Duration:** Complete
**Focus:** Convert JavaScript to TypeScript with explicit types

**Completed:**
- ✅ Set up TypeScript configuration (tsconfig.json, vite.config.ts)
- ✅ Added TypeScript and Vite development dependencies
- ✅ Defined comprehensive type interfaces for WebSocket protocol
- ✅ Converted app.js to TypeScript (src/main.ts) with explicit types
- ✅ Implemented strict type safety for all functions and methods
- ✅ Added proper error handling with typed error messages
- ✅ Updated HTML to reference TypeScript module
- ✅ Built and tested TypeScript application successfully

**TypeScript Features Added:**
- **Strict Type Checking**: All functions have explicit parameter and return types
- **Interface Definitions**: Complete type definitions for WebSocket protocol
- **Union Types**: Proper typing for Content, Command, Notification variants
- **Generic Type Safety**: RequiredElement<T> for DOM element type safety
- **Error Handling**: Typed error handling with proper error message extraction
- **Configuration**: Typed configuration object with readonly properties

**File Structure:**
```
toboggan-web/
├── plan.md              # Implementation plan
├── devlog.md            # This development log
├── index.html           # HTML with TypeScript module reference
├── package.json         # Dependencies and scripts
├── tsconfig.json        # TypeScript configuration
├── vite.config.ts       # Vite build configuration
└── src/
    ├── types.ts         # Complete type definitions
    └── main.ts          # TypeScript application with explicit types
```

**Development Tools:**
- Vite for development server and building
- TypeScript with strict mode enabled
- Module system with proper imports/exports

**Type Safety Highlights:**
- All WebSocket message types properly defined
- DOM element access with required element checking
- Configuration with readonly properties
- Error handling with proper type guards
- Event handling with typed event objects

**Testing Results:**
- ✅ TypeScript compilation successful
- ✅ Vite build process working
- ✅ Development server running on port 8000
- ✅ All original functionality preserved with type safety

**Application now ready for complex development with full type safety.**

---

### Session 3: 2025-07-20 - Fix Slide Ordering
**Duration:** Complete
**Focus:** Fix slide ordering issue

**Issue:** Slides were not displayed in correct numerical order because object keys don't guarantee order in JavaScript.

**Solution Implemented:**
- Added `SlidesCache` interface to store both slides and ordered IDs
- Sort slide IDs numerically when fetching: `Object.keys().map(parseInt).sort((a,b) => a - b)`
- Cache slides data to avoid re-fetching and re-sorting
- Update slide counter to show correct position based on ordered array
- Clear cache on WebSocket disconnect to ensure fresh data

**Code Changes:**
```typescript
// Added cache structure
private slidesCache: SlidesCache | null = null;

// Sort slides on fetch
const orderedIds = Object.keys(data.slides)
  .map(id => parseInt(id, 10))
  .sort((a, b) => a - b);

// Display correct slide number
const currentIndex = this.slidesCache.orderedIds.indexOf(this.currentSlide);
const displayNumber = currentIndex >= 0 ? currentIndex + 1 : this.currentSlide + 1;
```

**Results:**
- ✅ Slides now display in correct numerical order (0, 1, 2, 3...)
- ✅ Slide counter shows accurate position (1/8, 2/8, etc.)
- ✅ Performance improved with caching
- ✅ Cache properly cleared on disconnect

**Slides now navigate in proper sequential order.**

---

### Session 4: 2025-07-20 - Add Duration Display
**Duration:** Complete
**Focus:** Implement presentation duration display with auto-updating timer

**Requirements:**
- Display duration in `hh:mm:ss` format
- Auto-update every second when presentation is running
- Use server-provided duration as source of truth
- Stop updating when paused or done

**Implementation:**
1. **Added Duration Display to UI:**
   ```html
   <span id="duration-display" aria-live="polite">Duration: --:--:--</span>
   ```

2. **Duration State Management:**
   ```typescript
   private durationInterval: number | null = null;
   private presentationState: State | null = null;
   private startTime: Date | null = null;
   ```

3. **Duration Calculation Logic:**
   - For Running state: Calculate start time from `since - total_duration`
   - For Paused/Done states: Display static `total_duration` from server
   - Auto-update every second using `setInterval` when running

4. **Format Helper:**
   ```typescript
   private formatDuration(totalSeconds: number): string {
     const hours = Math.floor(totalSeconds / 3600);
     const minutes = Math.floor((totalSeconds % 3600) / 60);
     const seconds = Math.floor(totalSeconds % 60);
     return `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;
   }
   ```

**Key Features:**
- ✅ Duration displays in `hh:mm:ss` format (e.g., "00:05:23")
- ✅ Timer auto-updates every second when presentation is running
- ✅ Timer stops when presentation is paused or done
- ✅ Server duration values used as source of truth
- ✅ Timer cleared on WebSocket disconnect
- ✅ Proper cleanup prevents memory leaks

**Results:**
- Duration display accurately tracks presentation time
- Smooth updates every second during playback
- Correctly preserves duration when paused
- Shows final duration when presentation is done

**Presentation now has complete time tracking functionality.**

---

### Session 5: 2025-07-20 - Internationalized Duration Formatting
**Duration:** Complete
**Focus:** Replace custom duration formatting with Intl.DateTimeFormat

**Changes:**
- Replaced manual hours:minutes:seconds formatting with `Intl.DateTimeFormat`
- Uses browser's locale for proper internationalization
- Leverages built-in formatting capabilities

**Implementation:**
```typescript
private formatDuration(totalSeconds: number): string {
  // Create date at epoch + duration, using UTC to avoid timezone issues
  const date = new Date(totalSeconds * 1000);
  
  const formatter = new Intl.DateTimeFormat(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23', // 24-hour format
    timeZone: 'UTC'
  });
  
  return formatter.format(date);
}
```

**Benefits:**
- ✅ Automatic locale-appropriate formatting
- ✅ Consistent with browser's language settings
- ✅ Proper handling of different numbering systems
- ✅ Built-in support for RTL languages
- ✅ Less code to maintain

**Duration now displays with proper internationalization support.**

---

### Session 6: 2025-07-20 - Modern UUID Generation
**Duration:** Complete
**Focus:** Replace custom UUID generation with crypto.randomUUID()

**Changes:**
- Removed custom UUID generation function
- Uses Web Crypto API's `crypto.randomUUID()` method
- Modern, secure, and standardized approach

**Before:**
```typescript
private generateClientId(): ClientId {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c: string): string => {
    const r = Math.random() * 16 | 0;
    const v = c === 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}
```

**After:**
```typescript
private generateClientId(): ClientId {
  return crypto.randomUUID();
}
```

**Benefits:**
- ✅ Cryptographically secure random UUIDs
- ✅ Native browser API (no custom implementation)
- ✅ Guaranteed RFC 4122 compliant UUIDs
- ✅ Better performance
- ✅ Less code to maintain
- ✅ Smaller bundle size

**Browser Support:**
- Supported in all modern browsers
- Chrome 92+, Firefox 95+, Safari 15.4+

**Application now uses modern Web APIs throughout.**

---

### Session 7: 2025-07-20 - Modular Architecture Refactoring
**Duration:** Complete
**Focus:** Split monolithic main.ts into small, focused modules

**Motivation:**
- Improve code organization and maintainability
- Encapsulate implementation details
- Enable better testing and reusability
- Follow single responsibility principle

**Module Structure Created:**
```
src/
├── main.ts                    # Minimal entry point
├── types.ts                   # All TypeScript interfaces
├── controllers/
│   └── presentationController.ts  # Main coordination logic
├── services/
│   ├── websocket.ts          # WebSocket connection management
│   ├── slidesApi.ts          # Slides data fetching & caching
│   ├── slideRenderer.ts      # Slide content rendering
│   └── timer.ts              # Duration timer management
└── utils/
    ├── dom.ts                # DOM utilities
    └── duration.ts           # Duration formatting utilities
```

**Key Design Decisions:**

1. **Service Layer Pattern:**
   - Each service has a single responsibility
   - Services expose only public methods
   - Implementation details are private

2. **Dependency Injection:**
   - Controller receives all dependencies via constructor
   - No global state or singletons
   - Easy to test and mock

3. **Callback-Based Architecture:**
   - WebSocket service uses callbacks instead of events
   - Prevents tight coupling between modules
   - Clear data flow

4. **Encapsulation:**
   - Private methods hide implementation details
   - Public interfaces are minimal and focused
   - No leaking of internal state

**Module Breakdown:**

**Utils (Pure Functions):**
- `dom.ts`: Element access, HTML escaping, error display
- `duration.ts`: Time formatting, elapsed time calculations

**Services (Stateful Classes):**
- `WebSocketService`: Connection lifecycle, reconnection logic, message handling
- `SlidesApiService`: API calls, slide caching, ordering logic
- `SlideRenderer`: Content rendering, HTML generation
- `TimerService`: Duration tracking, interval management

**Controllers:**
- `PresentationController`: Coordinates all services, handles state changes

**Main App:**
- Minimal setup and configuration
- Event listener attachment
- Resource cleanup on unload

**Benefits Achieved:**
- ✅ Code is now highly modular and testable
- ✅ Each module has clear boundaries
- ✅ Implementation details are properly encapsulated
- ✅ Dependencies are explicit and injected
- ✅ Easy to extend or replace individual modules
- ✅ Better separation of concerns

**Bundle Size:** 10.33 kB (gzipped: 3.43 kB)

**Application architecture now follows best practices for maintainability and scalability.**