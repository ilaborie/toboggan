# Mobile avec UniFFI

```rust
// Logique métier partagée
pub struct UserService {
    // ...
}

impl UserService {
    pub fn authenticate(&self, token: String) -> Result<User, Error> {
        // ...
    }
}
```

- Code partagé iOS/Android
- Bindings automatiques Swift/Kotlin
- Utilisé par Mozilla Firefox