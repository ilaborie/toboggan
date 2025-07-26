import SwiftUI

// MARK: - State Extensions

extension State {
    var isPlaying: Bool {
        if case .running = self {
            return true
        }
        return false
    }
    
    var isPaused: Bool {
        if case .paused = self {
            return true
        }
        return false
    }
    
    var isDone: Bool {
        if case .done = self {
            return true
        }
        return false
    }
    
    var currentSlideId: String {
        switch self {
        case .running(let current, _),
             .paused(let current, _),
             .done(let current, _):
            return current
        }
    }
    
    var totalDuration: TimeInterval {
        let ms: UInt64
        switch self {
        case .running(_, let totalDurationMs),
             .paused(_, let totalDurationMs),
             .done(_, let totalDurationMs):
            ms = totalDurationMs
        }
        return TimeInterval(ms) / 1000.0
    }
}

// MARK: - SlideKind Extensions

extension SlideKind {
    var displayName: String {
        switch self {
        case .cover:
            return "Cover"
        case .part:
            return "Part"
        case .standard:
            return "Standard"
        }
    }
    
    var systemImage: String {
        switch self {
        case .cover:
            return "star.fill"
        case .part:
            return "folder.fill"
        case .standard:
            return "doc.text.fill"
        }
    }
    
    var color: Color {
        switch self {
        case .cover:
            return .purple
        case .part:
            return .blue
        case .standard:
            return .green
        }
    }
}

// MARK: - SlideStyle Extensions

extension SlideStyle {
    var textAlignment: TextAlignment {
        switch self {
        case .center:
            return .center
        case .default, .code:
            return .leading
        }
    }
    
    var font: Font {
        switch self {
        case .code:
            return .system(.body, design: .monospaced)
        case .default, .center:
            return .body
        }
    }
}

// MARK: - TimeInterval Extensions

extension TimeInterval {
    var formattedDuration: String {
        let minutes = Int(self) / 60
        let seconds = Int(self) % 60
        return String(format: "%d:%02d", minutes, seconds)
    }
}