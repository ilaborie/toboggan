import Foundation

struct AppConstants {
    static let defaultServerURL = "ws://localhost:3000/api/ws"
    static let maxRetries: UInt32 = 5
    static let retryDelayMs: UInt64 = 1000
    static let connectionTimeoutSeconds: Double = 10.0
}

struct UIConstants {
    static let slideCornerRadius: CGFloat = 12
    static let controlButtonSize: CGFloat = 44
    static let playButtonSize: CGFloat = 60
    static let statusBarHeight: CGFloat = 44
}