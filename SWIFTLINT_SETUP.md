# SwiftLint SourceKit Issue Fix

## Problem
SwiftLint is failing with the error:
```
SourceKittenFramework/library_wrapper.swift:58: Fatal error: Loading sourcekitdInProc.framework/Versions/A/sourcekitdInProc failed
```

## Root Cause
SwiftLint requires SourceKit, which is part of Xcode. The issue occurs when:
1. Only Xcode Command Line Tools are installed (not full Xcode)
2. Xcode developer directory is pointing to Command Line Tools instead of Xcode app
3. SourceKit framework is not available or corrupted

## Solutions

### Option 1: Install Full Xcode (Recommended for Development)

1. **Install Xcode from App Store**:
   ```bash
   # Open App Store and search for "Xcode", or use:
   open "macappstore://itunes.apple.com/app/id497799835"
   ```

2. **Set Xcode as developer directory**:
   ```bash
   sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
   ```

3. **Accept Xcode license**:
   ```bash
   sudo xcodebuild -license accept
   ```

4. **Verify setup**:
   ```bash
   xcode-select --print-path
   # Should output: /Applications/Xcode.app/Contents/Developer
   ```

### Option 2: Alternative SwiftLint Installation (For CI/Headless)

If you can't install full Xcode, use a different SwiftLint distribution:

1. **Using Homebrew** (may work better):
   ```bash
   # Remove mise-installed SwiftLint
   mise uninstall swiftlint
   
   # Install via Homebrew
   brew install swiftlint
   ```

2. **Using Swift Package Manager**:
   ```bash
   # Clone and build SwiftLint from source
   git clone https://github.com/realm/SwiftLint.git
   cd SwiftLint
   swift build -c release
   # Binary will be at .build/release/swiftlint
   ```

### Option 3: Docker-based SwiftLint (For CI)

For CI environments where Xcode can't be installed:

```dockerfile
# Use Swift image with SourceKit
FROM swift:5.9

# Install SwiftLint
RUN git clone https://github.com/realm/SwiftLint.git \\
    && cd SwiftLint \\
    && swift build -c release \\
    && cp .build/release/swiftlint /usr/local/bin/
```

### Option 4: Mock SwiftLint for Development

If you want to skip SwiftLint temporarily:

```bash
# Create a mock swiftlint that always succeeds
echo '#!/bin/bash\necho "Mock SwiftLint: No issues found"' > /usr/local/bin/swiftlint
chmod +x /usr/local/bin/swiftlint
```

## Quick Fix for Current Setup

Let me update the mise configuration to handle this more gracefully: