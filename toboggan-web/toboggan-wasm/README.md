# Toboggan WASM Client

WebAssembly client for the Toboggan presentation system, providing real-time presentation control and display capabilities in web browsers. Built with Rust and compiled to WASM for performance and safety.

## Features

### Core Functionality
- **Real-time WebSocket Communication**: Bi-directional communication with Toboggan server
- **Presentation State Management**: Synchronized presentation state across all clients
- **Command Processing**: Handle navigation and control commands (Next, Previous, Play, Pause)
- **Error Handling**: Robust error handling with user-friendly error messages
- **Retry Logic**: Automatic reconnection with exponential backoff

### Web Integration
- **JavaScript Bindings**: Seamless integration with TypeScript/JavaScript frontends
- **Browser Compatibility**: Works in all modern browsers with WebAssembly support
- **Memory Efficiency**: Optimized WASM build with minimal memory footprint
- **Security**: XSS prevention through content sanitization
- **Performance**: Native-speed execution with Rust's zero-cost abstractions

### Development Features
- **Hot Reload Support**: Fast development cycles with live rebuilding
- **Debug Support**: Console logging and error reporting
- **TypeScript Definitions**: Full type safety for web development

## Architecture

### Core Components

```rust
// Main WASM interface
#[wasm_bindgen]
pub struct TobogganClient {
    inner: Arc<Mutex<ClientInner>>,
}

#[wasm_bindgen]
impl TobogganClient {
    #[wasm_bindgen(constructor)]
    pub fn new(server_url: String) -> TobogganClient;
    
    pub async fn connect(&self) -> Result<(), JsValue>;
    pub async fn send_command(&self, command: &str) -> Result<(), JsValue>;
    pub fn set_state_callback(&self, callback: js_sys::Function);
}
```

### Integration with Web Frontend

```typescript
import { TobogganClient } from './pkg/toboggan_wasm';

// Initialize client
const client = new TobogganClient('ws://localhost:8080/api/ws');

// Set up state updates
client.set_state_callback((state) => {
    console.log('Presentation state updated:', state);
    // Update UI based on new state
});

// Connect to server
await client.connect();

// Send commands
await client.send_command('{"type": "Next"}');
await client.send_command('{"type": "Pause"}');
```

## Development Workflow

### Live Development with Bacon

For rapid development cycles, use `bacon wasm` to automatically rebuild on changes:

```bash
# Start live build watcher
bacon wasm
```

**Benefits:**
- **Instant Feedback**: Immediate compilation results on file save
- **Error Reporting**: Clear error messages with source locations
- **Build Optimization**: Automatic `wasm-opt` optimization
- **Size Monitoring**: Track WASM bundle size changes

**Output:**
```
✓ Compiling toboggan-wasm
✓ Running wasm-pack build
✓ Optimizing with wasm-opt
📦 WASM size: 245KB → 189KB (optimized)
```

### Manual Build Process

#### Option 1: Using Mise (Recommended)
```bash
# Build optimized WASM package
mise build:wasm
```

#### Option 2: Direct wasm-pack Build
```bash
# Development build (faster, larger)
wasm-pack build --target web --dev

# Release build (optimized, smaller)
wasm-pack build --target web --release
```

#### Option 3: Custom Build with Optimization
```bash
# Build with maximum optimization
wasm-pack build --target web --release
wasm-opt pkg/toboggan_wasm_bg.wasm -O4 -o pkg/toboggan_wasm_bg_opt.wasm
```

### Build Configuration

The build is configured via `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = { version = "0.2", features = ["serde-serialize"] }
web-sys = { version = "0.3", features = ["console", "WebSocket"] }
js-sys = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde-wasm-bindgen = "0.6"

[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-O4", "--enable-mutable-globals"]
```

## Package Output

Built files are generated in the `pkg/` directory:

```
pkg/
├── toboggan_wasm.js              # JavaScript bindings and glue code
├── toboggan_wasm.d.ts            # TypeScript type definitions
├── toboggan_wasm_bg.wasm         # Main WASM binary
├── toboggan_wasm_bg_opt.wasm     # Optimized WASM binary
├── toboggan_wasm_bg.wasm.d.ts    # WASM-specific type definitions
├── package.json                  # NPM package configuration
└── README.md                     # Package documentation
```

### File Descriptions

- **`toboggan_wasm.js`**: Main entry point with JavaScript bindings
- **`toboggan_wasm.d.ts`**: TypeScript definitions for all exported functions
- **`toboggan_wasm_bg.wasm`**: WebAssembly binary with Rust logic
- **`toboggan_wasm_bg_opt.wasm`**: Size-optimized version (use in production)

## Integration Examples

### React Integration

```typescript
import React, { useEffect, useState } from 'react';
import { TobogganClient } from '../pkg/toboggan_wasm';

export function PresentationController() {
    const [client, setClient] = useState<TobogganClient | null>(null);
    const [state, setState] = useState<any>(null);
    
    useEffect(() => {
        const initClient = async () => {
            const wasmClient = new TobogganClient('ws://localhost:8080/api/ws');
            
            wasmClient.set_state_callback((newState) => {
                setState(JSON.parse(newState));
            });
            
            await wasmClient.connect();
            setClient(wasmClient);
        };
        
        initClient().catch(console.error);
    }, []);
    
    const sendCommand = async (command: string) => {
        if (client) {
            await client.send_command(JSON.stringify({ type: command }));
        }
    };
    
    return (
        <div>
            <button onClick={() => sendCommand('Next')}>Next</button>
            <button onClick={() => sendCommand('Previous')}>Previous</button>
            <button onClick={() => sendCommand('Pause')}>Pause</button>
            {state && <div>Current slide: {state.current}</div>}
        </div>
    );
}
```

### Vanilla JavaScript Integration

```javascript
import init, { TobogganClient } from './pkg/toboggan_wasm.js';

async function startPresentation() {
    // Initialize WASM module
    await init();
    
    // Create client
    const client = new TobogganClient('ws://localhost:8080/api/ws');
    
    // Set up state handler
    client.set_state_callback((state) => {
        const parsedState = JSON.parse(state);
        document.getElementById('slide-counter').textContent = 
            `Slide ${parsedState.current + 1}`;
    });
    
    // Connect to server
    try {
        await client.connect();
        console.log('Connected to Toboggan server');
    } catch (error) {
        console.error('Connection failed:', error);
    }
    
    // Set up navigation buttons
    document.getElementById('next-btn').addEventListener('click', () => {
        client.send_command('{"type": "Next"}');
    });
    
    document.getElementById('prev-btn').addEventListener('click', () => {
        client.send_command('{"type": "Previous"}');
    });
}

startPresentation().catch(console.error);
```

## Performance Optimization

### Build Optimization

```bash
# Enable all optimizations
export RUSTFLAGS="-C opt-level=s -C lto=fat -C codegen-units=1"

# Build with optimizations
wasm-pack build --target web --release

# Further optimize with wasm-opt
wasm-opt pkg/toboggan_wasm_bg.wasm -Os -o pkg/toboggan_wasm_bg_opt.wasm
```

### Bundle Size Optimization

Current optimized sizes:
- **WASM Binary**: ~180KB (gzipped: ~65KB)
- **JavaScript Glue**: ~15KB (gzipped: ~5KB)
- **Total Package**: ~195KB (gzipped: ~70KB)

**Size Reduction Techniques:**
- Conditional compilation for debug features
- Minimal external dependencies
- Efficient serialization with `serde`
- Dead code elimination with `wee_alloc`

### Runtime Performance

- **Zero-cost abstractions**: Rust's performance guarantees
- **Memory efficiency**: Manual memory management via `wasm-bindgen`
- **Minimal JavaScript overhead**: Direct WASM function calls
- **Async/await support**: Non-blocking WebSocket operations

## Testing

### Unit Tests

```bash
# Run Rust unit tests
cargo test

# Test WASM bindings
wasm-pack test --headless --firefox
```

### Browser Testing

```bash
# Test in Chrome
wasm-pack test --headless --chrome

# Test in Firefox
wasm-pack test --headless --firefox

# Interactive testing in browser
wasm-pack test --firefox
```

### Integration Testing

```javascript
// Example integration test
describe('Toboggan WASM Client', () => {
    let client;
    
    beforeAll(async () => {
        await init(); // Initialize WASM
        client = new TobogganClient('ws://localhost:8080/api/ws');
    });
    
    test('should connect to server', async () => {
        await expect(client.connect()).resolves.toBeUndefined();
    });
    
    test('should send commands', async () => {
        await expect(
            client.send_command('{"type": "Next"}')
        ).resolves.toBeUndefined();
    });
});
```

## Debugging

### Development Setup

```rust
// Enable console logging in development
#[cfg(feature = "console_error_panic_hook")]
console_error_panic_hook::set_once();

// Log to browser console
web_sys::console::log_1(&"WASM client initialized".into());
```

### Browser DevTools

```javascript
// Enable WASM debugging in Chrome DevTools
// 1. Open DevTools
// 2. Go to Settings > Experiments
// 3. Enable "WebAssembly Debugging"
// 4. Restart DevTools

// Debug WASM in the Sources panel
debugger; // JavaScript breakpoint
// WASM breakpoints available in Sources > wasm://
```

### Error Handling

```rust
// Convert Rust errors to JavaScript
#[wasm_bindgen]
pub fn risky_operation() -> Result<String, JsValue> {
    match some_fallible_operation() {
        Ok(result) => Ok(result),
        Err(e) => Err(JsValue::from_str(&format!("Operation failed: {}", e)))
    }
}
```

## Contributing

### Development Setup

```bash
# Install dependencies
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# Clone and setup
git clone https://github.com/ilaborie/toboggan
cd toboggan/toboggan-web/toboggan-wasm

# Start development
bacon wasm  # Live rebuild
```

### Code Guidelines

- **Safety First**: No `unsafe` code (enforced by workspace lints)
- **Error Handling**: Comprehensive error propagation to JavaScript
- **Performance**: Minimize allocations and optimize hot paths
- **Compatibility**: Test across major browsers (Chrome, Firefox, Safari)
- **Documentation**: Document all public APIs with examples

### Testing Requirements

- All public functions must have unit tests
- Browser compatibility tests required for new features
- Integration tests for WebSocket functionality
- Performance benchmarks for critical paths

The WASM client provides a high-performance, memory-safe foundation for web-based Toboggan presentation clients.