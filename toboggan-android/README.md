# Toboggan Android App

Android client for the Toboggan presentation remote control system, built with Kotlin and Jetpack Compose.

## Prerequisites

1. **Android Studio** (latest version with Kotlin 2.0+ support)
2. **Android NDK** (version 25 or higher)
3. **mise** with the project's `.mise.toml` configured (provides `cargo-ndk` and Android SDK env vars)
4. **Rust toolchain** with Android targets:
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android
   ```

## Building

### 1. Build the Rust native library

From the project root directory:

```bash
mise build:android
```

This task will:
- Build the Rust library for all Android architectures (arm64-v8a, armeabi-v7a, x86_64, x86)
- Generate Kotlin bindings using UniFFI
- Copy the `.so` files to `app/src/main/jniLibs/`
- Copy the generated Kotlin bindings to `app/src/main/java/`

### 2. Open in Android Studio

Open the `toboggan-android/` folder in Android Studio.

### 3. Build and Run

Build and run the app on an emulator or device.

## Architecture

- **UI**: Jetpack Compose with Material 3
- **State Management**: ViewModel with StateFlow
- **Native Integration**: UniFFI-generated Kotlin bindings to Rust

## Project Structure

```
toboggan-android/
├── app/src/main/
│   ├── java/com/toboggan/app/
│   │   ├── MainActivity.kt
│   │   ├── TobogganApplication.kt
│   │   ├── ui/
│   │   │   ├── components/      # Reusable UI components
│   │   │   ├── screens/         # Screen composables
│   │   │   └── theme/           # Material 3 theming
│   │   └── viewmodel/           # ViewModels
│   ├── jniLibs/                 # Native .so libraries (generated)
│   └── res/                     # Android resources
└── gradle/                      # Gradle configuration
```

## Development Notes

- The server URL is hardcoded to `10.0.2.2:8080` for Android emulator (maps to host's localhost)
- For physical device testing, update the URL in `PresentationViewModel.kt`
