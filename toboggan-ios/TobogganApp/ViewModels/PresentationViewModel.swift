import SwiftUI
import Combine

enum ConnectionStatus {
    case disconnected
    case connecting
    case connected
    case error(String)
}

@MainActor
class PresentationViewModel: ObservableObject {
    @Published var connectionStatus: ConnectionStatus = .disconnected
    @Published var errorMessage: String?
    @Published var currentSlide: Slide?
    @Published var talkInfo: TalkInfo?
    @Published var presentationState: State?
    
    private var tobogganClient: TobogganClient?
    
    var isConnected: Bool {
        if case .connected = connectionStatus {
            return true
        }
        return false
    }
    
    var isConnecting: Bool {
        if case .connecting = connectionStatus {
            return true
        }
        return false
    }
    
    func connect(to url: String) async {
        connectionStatus = .connecting
        errorMessage = nil
        
        do {
            let config = ClientConfig(
                websocketUrl: url,
                maxRetries: 5,
                retryDelayMs: 1000
            )
            
            tobogganClient = try createClient(config: config)
            
            try await tobogganClient?.connect()
            connectionStatus = .connected
            
            // Load initial data
            await loadTalkInfo()
            await loadCurrentState()
            
        } catch {
            errorMessage = "Failed to connect: \\(error.localizedDescription)"
            connectionStatus = .error(error.localizedDescription)
        }
    }
    
    func disconnect() async {
        do {
            try await tobogganClient?.disconnect()
            connectionStatus = .disconnected
            clearData()
        } catch {
            errorMessage = "Failed to disconnect: \\(error.localizedDescription)"
        }
    }
    
    func sendCommand(_ command: Command) {
        do {
            try tobogganClient?.sendCommand(command: command)
        } catch {
            errorMessage = "Failed to send command: \\(error.localizedDescription)"
        }
    }
    
    func nextSlide() {
        sendCommand(.next)
    }
    
    func previousSlide() {
        sendCommand(.previous)
    }
    
    func firstSlide() {
        sendCommand(.first)
    }
    
    func lastSlide() {
        sendCommand(.last)
    }
    
    func playPresentation() {
        sendCommand(.play)
    }
    
    func pausePresentation() {
        sendCommand(.pause)
    }
    
    func resumePresentation() {
        sendCommand(.resume)
    }
    
    func clearError() {
        errorMessage = nil
    }
    
    private func loadTalkInfo() async {
        talkInfo = await tobogganClient?.getTalkInfo()
    }
    
    private func loadCurrentState() async {
        presentationState = await tobogganClient?.getCurrentState()
        
        if let state = presentationState {
            let slideId = getCurrentSlideId(from: state)
            currentSlide = await tobogganClient?.getSlide(slideId: slideId)
        }
    }
    
    private func getCurrentSlideId(from state: State) -> String {
        switch state {
        case .running(let current, _),
             .paused(let current, _),
             .done(let current, _):
            return current
        }
    }
    
    private func clearData() {
        currentSlide = nil
        talkInfo = nil
        presentationState = nil
        tobogganClient = nil
    }
}

// Mock types for development - these will be replaced by generated UniFFI types
#if DEBUG
extension PresentationViewModel {
    static var preview: PresentationViewModel {
        let vm = PresentationViewModel()
        vm.connectionStatus = .connected
        vm.talkInfo = TalkInfo(
            title: "Sample Presentation",
            date: "2025-01-26",
            slides: [
                Slide(
                    id: "1",
                    title: "Welcome",
                    body: "Welcome to Toboggan",
                    kind: .cover,
                    style: .default,
                    notes: "Opening remarks"
                ),
                Slide(
                    id: "2",
                    title: "Introduction",
                    body: "Let's get started with our presentation",
                    kind: .standard,
                    style: .default,
                    notes: nil
                )
            ]
        )
        vm.currentSlide = vm.talkInfo?.slides.first
        vm.presentationState = .paused(current: "1", totalDurationMs: 0)
        return vm
    }
}
#endif