# Peut-on RIIR de tout ?

> Rewriting It In Rust - De la startup aux multinationales

______________________________________________________________________

## Introduction

**RIIR** : "Have you considered Rewriting It In Rust?"

Une question qui fait sourire… mais qui cache une réalité : Rust gagne du terrain partout.

______________________________________________________________________

## 1. Les Success Stories du RIIR

Des réécritures qui ont fait leurs preuves

Pourquoi ces réécritures réussissent ?

- **ripgrep** (`rg`) : grep réécrit en Rust

  - 10x plus rapide que grep classique
  - Recherche récursive native
  - Support Unicode complet

- **fd** : find réécrit en Rust

  - Interface plus intuitive
  - Performances supérieures
  - Respect des .gitignore par défaut

- **Fish Shell** : Shell moderne

  - Autocomplétion intelligente
  - Sécurité mémoire
  - Configuration simple

- **Performance** : Compilation native + optimisations

- **Sécurité** : Zéro segfault, gestion mémoire automatique

- **Ergonomie** : APIs modernes et intuitives

- **Fiabilité** : System de types expressif

______________________________________________________________________

## 2. Rust, le couteau suisse moderne

Au-delà des outils CLI

Les forces de Rust

Rust ne se limite pas aux applications terminal :

#### Web & Backend

- **Actix-web**, **Axum** : Serveurs web haute performance
- **Diesel**, **SQLx** : ORMs type-safe
- **Tokio** : Runtime async de référence

#### Applications Desktop

- **Tauri** : Alternative à Electron
- **egui**, **iced** : GUI natives
- **Bevy** : Moteur de jeu en ECS

#### Microcontrôleurs & IoT

- **Embassy** : Framework async pour embedded
- Support natif ARM, RISC-V
- Consommation mémoire optimisée

#### Blockchain & Crypto

- **Solana** : Runtime blockchain
- **Substrate** : Framework pour blockchains
- Performances critiques + sécurité

1. **Zero-cost abstractions** : Performance sans compromis
2. **Memory safety** : Pas de garbage collector, pas de segfault
3. **Concurrence** : Ownership model + async/await
4. **Écosystème** : Cargo + crates.io
5. **Cross-platform** : Linux, macOS, Windows, WASM, mobile

______________________________________________________________________

## 3. Rust s'intègre partout

WebAssembly (WASM)

Python avec PyO3 + Maturin

Mobile avec UniFFI

Autres intégrations

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

```rust
use pyo3::prelude::*;

#[pyfunction]
fn compute_heavy_task(data: Vec<f64>) -> PyResult<f64> {
    // Calculs intensifs en Rust
    Ok(data.iter().sum())
}

#[pymodule]
fn mymodule(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compute_heavy_task, m)?)?;
    Ok(())
}
```

- Accélération des parties critiques
- Distribution via pip
- Exemples : Pydantic v2, Polars

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

- **Node.js** : NAPI-RS

- **Ruby** : magnus, rutie

- **C/C++** : FFI direct

- **Java** : JNI

- **Go** : CGO

______________________________________________________________________

## 4. Rust en startup : Retour d'expérience

Pourquoi choisir Rust en startup ?

Stratégie d'adoption progressive

Success stories startup

#### Avantages

- **Performance** : Moins de serveurs = coûts réduits
- **Fiabilité** : Moins de bugs en production
- **Productivité** : Détection d'erreurs à la compilation
- **Évolutivité** : Refactoring sûr et confiant

#### Défis

- **Courbe d'apprentissage** : Concepts ownership/borrowing
- **Écosystème** : Plus jeune que Java/.NET
- **Recrutement** : Développeurs Rust plus rares

1. **Microservices critiques** : Performance-sensitive
2. **Outils internes** : CLI, scripts automation
3. **Extensions** : Plugins Python/Node.js
4. **Migration graduelle** : Module par module

- **Discord** : Backend haute performance
- **Dropbox** : Storage engine
- **Figma** : Moteur de rendu WASM
- **Vercel** : Bundlers (SWC, Turbo)

______________________________________________________________________

## Conclusion

RIIR : Pas qu'un mème

Quand envisager Rust ?

Le futur est rouillé ? 🦀

- **Réalité technique** : Gains mesurables performance/fiabilité
- **Écosystème mature** : Outils production-ready
- **Adoption croissante** : Startups → GAFAM

✅ **OUI** pour :

- Performance critique
- Sécurité prioritaire
- Code partagé multi-plateformes
- Outils système

❌ **NON** pour :

- Prototypage rapide
- Équipe junior exclusive
- Deadline très serrée
- Domain métier complexe

Rust n'est pas la solution à tout, mais il repousse les limites du possible.

**Question finale** : *"Have you considered Rewriting It In Rust?"*

Peut-être que la réponse n'est plus si farfelue…

______________________________________________________________________

## Ressources

*Merci pour votre attention !*

- [Rust Book](https://doc.rust-lang.org/book/)
- [RIIR repository](https://github.com/ansuz/RIIR)
- [Are we X yet?](https://wiki.mozilla.org/Areweyet)
- [This Week in Rust](https://this-week-in-rust.org/)
