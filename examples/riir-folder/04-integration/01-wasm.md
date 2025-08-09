# WebAssembly (WASM)

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn process_data(input: &str) -> String {
    // Logique métier en Rust
    format!("Processed: {}", input)
}
```

- Performance native dans le navigateur
- Interopérabilité JavaScript seamless
- Utilisé par Figma, Discord, Dropbox