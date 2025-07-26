# Swift Linting with SwiftLint

This document describes the Swift linting setup for the Toboggan iOS project using SwiftLint.

## Overview

SwiftLint has been integrated into the Toboggan project's mise-based development workflow to ensure consistent Swift code quality alongside Rust code quality checks.

## Setup

### Installation

SwiftLint is automatically managed through mise. To install:

```bash
mise install swiftlint
```

Or if you prefer to install manually:

```bash
# macOS
brew install swiftlint

# Other methods
# See: https://github.com/realm/SwiftLint#installation
```

### Configuration

SwiftLint is configured via `.swiftlint.yml` in the project root with:

- **Strict Rules**: Comprehensive set of opt-in rules for high code quality
- **Reasonable Limits**: Balanced line length (120 warning, 150 error) and function complexity
- **iOS-Specific**: Optimized for SwiftUI and iOS development patterns
- **Custom Rules**: Project-specific rules for force unwrapping, error handling, and TODO comments

## Usage

### Development Workflow

1. **Check**: Run all quality checks including SwiftLint
   ```bash
   mise run check
   ```

2. **Lint**: Run only linting (Rust + Swift)
   ```bash
   mise run lint
   ```

3. **Fix**: Auto-fix formatting and simple linting issues
   ```bash
   mise run fix
   ```

### Manual SwiftLint Commands

```bash
# Lint Swift code
swiftlint lint toboggan-ios/TobogganApp --config .swiftlint.yml

# Auto-fix Swift issues
swiftlint --fix toboggan-ios/TobogganApp --config .swiftlint.yml

# List all rules
swiftlint rules

# Generate docs for rules
swiftlint generate-docs
```

## Rules Configuration

### Enabled Rules

The configuration enables a comprehensive set of rules including:

- **Code Quality**: `cyclomatic_complexity`, `function_body_length`, `nesting`
- **Style Consistency**: `identifier_name`, `line_length`, `type_name`
- **Best Practices**: `force_unwrapping`, `missing_docs`, `redundant_type_annotation`
- **Modern Swift**: `implicit_return`, `trailing_closure`, `contains_over_filter_count`

### Custom Rules

1. **No Force Unwrapping in Production**: Warns against `!` usage
2. **Discourage try!**: Promotes proper error handling
3. **TODO Requires Context**: Ensures TODO comments have descriptions

### Exclusions

- Build artifacts (`target/`, `.build/`, `build/`)
- Xcode project files (`.xcodeproj`, `.xcworkspace`)
- Short identifier names (`id`, `to`, `vm`) are allowed

## Integration with CI/CD

The SwiftLint configuration includes:

```yaml
reporter: "github-actions-logging"
```

This provides optimal output formatting for GitHub Actions and other CI systems.

## File Organization

SwiftLint checks the following structure:

```
toboggan-ios/
├── .swiftlint.yml              # Project root configuration
└── TobogganApp/                # Swift source files
    ├── App/
    ├── ViewModels/
    ├── Views/
    ├── Services/
    ├── Utils/
    └── Resources/
```

## Common Issues and Solutions

### 1. Line Length Warnings

```swift
// ❌ Too long
let veryLongVariableName = someFunction(withParameter: parameter, andAnotherParameter: anotherParameter, andYetAnotherParameter: yetAnotherParameter)

// ✅ Better
let veryLongVariableName = someFunction(
    withParameter: parameter,
    andAnotherParameter: anotherParameter,
    andYetAnotherParameter: yetAnotherParameter
)
```

### 2. Force Unwrapping

```swift
// ❌ Dangerous
let value = optionalValue!

// ✅ Safe
guard let value = optionalValue else { return }
// or
if let value = optionalValue {
    // use value
}
```

### 3. Missing Documentation

```swift
// ❌ Missing docs for public API
public func importantFunction() { }

// ✅ Documented
/// Performs an important operation
/// - Returns: The result of the operation
public func importantFunction() -> Result { }
```

### 4. Error Handling

```swift
// ❌ Dangerous
let data = try! decoder.decode(Model.self, from: jsonData)

// ✅ Proper error handling
do {
    let data = try decoder.decode(Model.self, from: jsonData)
    // use data
} catch {
    // handle error
}
```

## Xcode Integration

To integrate SwiftLint with Xcode:

1. Add a Run Script Phase to your Xcode project
2. Set the script content to:
   ```bash
   if command -v swiftlint >/dev/null 2>&1; then
       swiftlint lint --config "${SRCROOT}/.swiftlint.yml"
   fi
   ```

This will show SwiftLint warnings and errors directly in Xcode.

## Performance Notes

- SwiftLint is only run when Swift files are present
- Configuration is optimized for fast linting
- Auto-fix is available for many rules to speed up development

## Updating Rules

To modify SwiftLint rules:

1. Edit `.swiftlint.yml` in the project root
2. Test with `swiftlint lint toboggan-ios/TobogganApp --config .swiftlint.yml`
3. Run `mise run check` to ensure integration works
4. Commit changes to version control

## Resources

- [SwiftLint Documentation](https://github.com/realm/SwiftLint)
- [SwiftLint Rule Directory](https://realm.github.io/SwiftLint/rule-directory.html)
- [Swift Style Guide](https://google.github.io/swift/)
- [iOS Development Best Practices](https://developer.apple.com/documentation/swift)