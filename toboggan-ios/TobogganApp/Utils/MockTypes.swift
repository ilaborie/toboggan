import Foundation

// MARK: - Mock types for development
// These will be replaced by generated UniFFI types when the build system is complete

// MARK: - Core Types

struct ClientConfig {
    let websocketUrl: String
    let maxRetries: UInt32
    let retryDelayMs: UInt64
}

enum TobogganError: Error, LocalizedError {
    case connectionError(message: String)
    case parseError(message: String)
    case configError(message: String)
    case unknownError(message: String)
    
    var errorDescription: String? {
        switch self {
        case .connectionError(let message):
            return "Connection error: \\(message)"
        case .parseError(let message):
            return "Parse error: \\(message)"
        case .configError(let message):
            return "Config error: \\(message)"
        case .unknownError(let message):
            return "Unknown error: \\(message)"
        }
    }
}

// MARK: - Slide Types

enum SlideKind {
    case cover
    case part
    case standard
}

enum SlideStyle {
    case `default`
    case center
    case code
}

struct Slide {
    let id: String
    let title: String
    let body: String
    let kind: SlideKind
    let style: SlideStyle
    let notes: String?
}

struct TalkInfo {
    let title: String
    let date: String?
    let slides: [Slide]
}

// MARK: - Control Types

enum Command {
    case next
    case previous
    case first
    case last
    case play
    case pause
    case resume
}

enum State {
    case running(current: String, totalDurationMs: UInt64)
    case paused(current: String, totalDurationMs: UInt64)
    case done(current: String, totalDurationMs: UInt64)
}

// MARK: - Client

class TobogganClient {
    private let config: ClientConfig
    private var isConnectedState = false
    
    init(config: ClientConfig) {
        self.config = config
    }
    
    func connect() async throws {
        // Simulate connection delay
        try await Task.sleep(nanoseconds: 500_000_000) // 0.5 seconds
        isConnectedState = true
    }
    
    func disconnect() async throws {
        isConnectedState = false
    }
    
    func sendCommand(command: Command) throws {
        guard isConnectedState else {
            throw TobogganError.connectionError(message: "Not connected")
        }
        
        // Mock command handling
        print("Sending command: \\(command)")
    }
    
    func getCurrentState() async -> State? {
        guard isConnectedState else { return nil }
        return .paused(current: "1", totalDurationMs: 0)
    }
    
    func getSlide(slideId: String) async -> Slide? {
        guard isConnectedState else { return nil }
        
        // Mock slide data
        let mockSlides = getMockSlides()
        return mockSlides.first { $0.id == slideId }
    }
    
    func getTalkInfo() async -> TalkInfo? {
        guard isConnectedState else { return nil }
        
        return TalkInfo(
            title: "Demo Presentation",
            date: "2025-01-26",
            slides: getMockSlides()
        )
    }
    
    func isConnected() async -> Bool {
        return isConnectedState
    }
    
    private func getMockSlides() -> [Slide] {
        return [
            Slide(
                id: "1",
                title: "Welcome to Toboggan",
                body: "A modern presentation system built with Rust and SwiftUI",
                kind: .cover,
                style: .center,
                notes: "Welcome everyone and introduce the topic"
            ),
            Slide(
                id: "2",
                title: "Getting Started",
                body: "Let's explore the features of this presentation system",
                kind: .standard,
                style: .default,
                notes: "Cover the main features briefly"
            ),
            Slide(
                id: "3",
                title: "Architecture",
                body: "The system uses a hybrid approach:\\n• Rust for core logic\\n• SwiftUI for native iOS UI\\n• UniFFI for seamless integration",
                kind: .part,
                style: .default,
                notes: "Explain the technical architecture"
            ),
            Slide(
                id: "4",
                title: "Code Example",
                body: "// Rust client creation\\nlet client = TobogganClient::new(config);\\nclient.connect().await?;",
                kind: .standard,
                style: .code,
                notes: nil
            )
        ]
    }
}

// MARK: - Factory Function

func createClient(config: ClientConfig) throws -> TobogganClient {
    return TobogganClient(config: config)
}