# Toboggan Server Refactoring Plan

## Overview
This plan outlines the refactoring needed in `toboggan-server` to handle the breaking changes made to `toboggan-core`.

## Breaking Changes in toboggan-core

### 1. Time Types - Type Aliases Instead of Wrappers
**Before**: `Timestamp` and `Date` were wrapper structs with methods
**After**: Type aliases for `jiff::Timestamp` and `jiff::civil::Date` with extension traits

**Impact**: 
- Direct usage of jiff types with utility functions in `date_utils` module
- `TimestampExt` trait for utility methods like `now()` and `elapsed()`

### 2. Content Type Simplification
**Before**: 
```rust
Content::Html { raw: String, alt: Option<String> }
Content::HBox { columns: String, contents: Vec<Content> }
Content::VBox { rows: String, contents: Vec<Content> }
```

**After**:
```rust
Content::Html { content: String }
// HBox and VBox removed
```

**Impact**:
- HTML serialization format changed
- Layout containers removed - layout handled by renderers
- Removed automatic file path conversions

### 3. State Enum Simplification
**Before**: 4 states (Init, Paused, Running, Done)
**After**: 3 states with Init merged into Paused
```rust
State::Paused { current: Option<SlideId>, total_duration: Duration }
State::Running { since: Timestamp, current: SlideId, total_duration: Duration }
State::Done { current: SlideId, total_duration: Duration }
```

**Impact**:
- Initial state is now `Paused { current: None, .. }`
- Navigation from initial state sets `current: Some(slide_id)`

### 4. SlideId Simplification
**Before**: Complex conditional compilation with Arc wrapping
**After**: Simple `AtomicU8` usage
**Impact**: Minimal - mostly internal changes

## Server Refactoring Tasks

### Phase 1: Update Imports and Basic Types
1. Update imports to use jiff types directly
2. Add `TimestampExt` import where needed
3. Update `date_utils` usage for Date creation

### Phase 2: State Handling Updates
1. Update state initialization logic
2. Handle `Option<SlideId>` in Paused state
3. Update command handling for new state structure
4. Update presentation manager state transitions

### Phase 3: Content Serialization Updates
1. Update HTML content handling for new field name
2. Remove layout container handling
3. Update any tests or examples using old Content format

### Phase 4: API Response Updates
1. Ensure JSON serialization works with new formats
2. Update OpenAPI schema if needed
3. Test WebSocket message format compatibility

## Implementation Priority

### High Priority (Breaking Changes)
1. **State enum changes** - Core to server functionality
2. **Content field name changes** - Affects serialization
3. **Time type imports** - Required for compilation

### Medium Priority
1. Remove layout container support from renderers
2. Update documentation and examples

### Low Priority
1. Optimize for simplified types
2. Remove unused imports/dependencies

## Testing Strategy

1. **Unit Tests**: Update existing tests for new State structure
2. **Integration Tests**: Verify multi-client sync still works
3. **API Tests**: Ensure JSON format compatibility
4. **WebSocket Tests**: Verify message format unchanged

## Migration Notes for Clients

### For Web/WASM Clients:
- Update Content.Html field from `raw` to `content`
- Remove layout container handling
- Handle initial state as Paused with no current slide

### For Desktop/CLI Clients:
- Similar Content updates
- Update state handling logic

## Success Criteria

1. `cargo build` succeeds for toboggan-server
2. All existing tests pass
3. `mise check` passes (format, lint, test)
4. Integration tests for multi-client sync work
5. WebSocket API maintains compatibility
6. Server starts and serves talks correctly

## Backward Compatibility Notes

- JSON format changes are breaking for existing clients
- WebSocket message format may need versioning
- Consider adding migration guide for client updates

## Files to Update

### Core Server Files:
- `src/state.rs` - Update for new State enum
- `src/router/ws.rs` - Handle state changes and commands
- `src/router/responses.rs` - Update response serialization

### Test Files:
- `tests/multi_client_sync.rs` - Update for new State format
- `tests/state_serialization.rs` - Update serialization tests
- `src/state_tests.rs` - Update state transition tests

### Configuration:
- `talk.toml` examples may need Content updates
- OpenAPI schema updates

## Implementation Steps

1. **Compile Fixes** (immediate)
   - Fix imports and basic type usage
   - Get server compiling again

2. **State Logic** (core functionality)
   - Update command handling
   - Fix state transitions
   - Test basic navigation

3. **API Compatibility** (client compatibility)
   - Verify JSON serialization
   - Test WebSocket messages
   - Update OpenAPI if needed

4. **Testing & Validation** (quality assurance)
   - Run all tests
   - Integration testing
   - Performance validation