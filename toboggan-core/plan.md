# Toboggan Core Simplification Plan

## Overview
This plan outlines the simplification of `toboggan-core` to reduce overengineering and improve maintainability. These are breaking changes that will require updates to `toboggan-server` and other dependent crates.

## Phase 1: Core Simplifications

### 1.1 SlideId Simplification
**Current**: Complex conditional compilation with `Arc<AtomicU8>` when `alloc` is available
**Target**: Single implementation using `AtomicU8` directly

```rust
// Before: Complex conditional compilation
#[cfg(feature = "alloc")]
static ID_SEQ: Lazy<Arc<AtomicU8>> = Lazy::new(Arc::default);
#[cfg(not(feature = "alloc"))]
static ID_SEQ: AtomicU8 = AtomicU8::new(0);

// After: Simple direct usage
static ID_SEQ: AtomicU8 = AtomicU8::new(0);
```

### 1.2 Content Type Simplification
**Current**: 7 variants with layout containers and file conversions
**Target**: 4 core variants without layout complexity

```rust
// Simplified Content enum
pub enum Content {
    Empty,
    Text { text: String },
    Html { content: String }, // Removed 'raw' and 'alt' fields
    IFrame { url: String },
    #[cfg(feature = "std")]
    Term { cwd: PathBuf },
}
```

**Remove**:
- `HBox` and `VBox` layout containers (let renderers handle layout)
- Automatic file path conversions
- Separate alt text for HTML (can be embedded in HTML if needed)

### 1.3 Direct Type Usage
**Current**: Wrapper types for `Timestamp` and `Date`
**Target**: Use `jiff` types directly via type aliases

```rust
// Type aliases instead of wrappers
pub type Timestamp = jiff::Timestamp;
pub type Date = jiff::civil::Date;
```

### 1.4 State Machine Simplification
**Current**: 4 states including `Init`
**Target**: 3 states with `Option<SlideId>`

```rust
pub enum State {
    Paused {
        current: Option<SlideId>, // None represents initial state
        total_duration: Duration,
    },
    Running {
        since: Timestamp,
        current: SlideId,
        total_duration: Duration,
    },
    Done {
        current: SlideId,
        total_duration: Duration,
    },
}
```

### 1.5 Feature Flag Reduction
**Current**: 5+ feature flags with complex interactions
**Target**: 2-3 essential flags

- Keep: `std` (default), `openapi`
- Remove: `alloc` (make it implicit with no_std), `js` (use target detection), `test-utils`
- Simplify: Use `#[cfg(test)]` instead of `test-utils` feature

### 1.6 ClientId Simplification
**Current**: Complex UUID generation with multiple paths
**Target**: Simple counter-based ID for presentations

```rust
pub struct ClientId(u32);

impl ClientId {
    pub fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}
```

## Phase 2: API Improvements

### 2.1 Builder Pattern Consistency
- Keep builder methods but simplify internal implementation
- Remove `with_style` when `alloc` not available (just use empty Style)

### 2.2 Serialization
- Simplify serde attributes
- Remove unnecessary tag attributes where possible

## Phase 3: Server Refactoring

### 3.1 Update Imports
- Change from wrapper types to direct jiff types
- Update Content variant usage

### 3.2 State Handling
- Update for new State enum without Init
- Handle `Option<SlideId>` in Paused state

### 3.3 Client Management
- Update for new ClientId implementation
- Simplify client tracking

## Implementation Order

1. **SlideId changes** (lowest impact)
2. **Time type aliases** (mechanical change)
3. **ClientId simplification** (isolated to command handling)
4. **Content type changes** (higher impact, affects serialization)
5. **State machine changes** (highest impact, affects core logic)
6. **Feature flag cleanup** (build system changes)

## Testing Strategy

1. Update existing tests for new APIs
2. Ensure serialization compatibility where needed
3. Test no_std builds still work
4. Verify WASM compilation

## Migration Notes

### For toboggan-server:
- Import `jiff::Timestamp` and `jiff::civil::Date` directly
- Update Content matching for removed variants
- Handle new State structure with Option<SlideId>
- Update ClientId usage

### For toboggan-wasm:
- Update for Content changes
- Handle simplified ClientId

### For other clients:
- Similar updates as above
- May need to implement own layout logic for removed HBox/VBox

## Success Criteria

1. `mise check` passes for all crates
2. Reduced lines of code in toboggan-core
3. Simpler API surface
4. All tests passing
5. WASM and no_std builds working