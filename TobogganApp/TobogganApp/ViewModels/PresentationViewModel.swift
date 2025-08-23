//
//  PresentationViewModel.swift
//  TobogganApp
//
//  Created by Igor Laborie on 16/08/2025.
//

import SwiftUI

// MARK: - Connection Status Extension
extension ConnectionStatus {
    var displayText: String {
        switch self {
        case .connecting:
            return "Connecting..."
        case .connected:
            return "Connected"
        case .closed:
            return "Disconnected"
        case .reconnecting:
            return "Reconnecting..."
        case .error:
            return "Connection Error"
        }
    }
    
    var color: Color {
        switch self {
        case .connected:
            return .green
        case .connecting, .reconnecting:
            return .orange
        default:
            return .red
        }
    }
}

// Notification handler implementation
final class NotificationHandler: ClientNotificationHandler, @unchecked Sendable {
    weak var viewModel: PresentationViewModel?
    
    init(viewModel: PresentationViewModel?) {
        self.viewModel = viewModel
    }
    
    func onStateChange(state: State) {
        viewModel?.handleStateChange(state)
    }
    
    func onConnectionStatusChange(status: ConnectionStatus) {
        viewModel?.handleConnectionStatusChange(status)
    }
    
    func onError(error: String) {
        viewModel?.handleError(error)
    }
}

// Shared configuration for TobogganClient
struct TobogganConfig {
    static let shared = ClientConfig(
        url: "http://127.0.0.1:8080",
        maxRetries: 3,
        retryDelay: 1.0
    )
}

// ViewModel following KISS principle with TobogganCore integration
class PresentationViewModel: ObservableObject {
    @Published var presentationTitle = "Presentation title - Date"
    @Published var nextSlideTitle = "Next Title"
    @Published var isPlaying = false
    @Published var connectionStatus: ConnectionStatus = .closed
    @Published var currentSlideIndex: Int? = nil
    @Published var totalSlides: Int = 0
    @Published var currentSlide: Slide?
    @Published var duration: TimeInterval = 0
    @Published var canGoPrevious = false
    @Published var canGoNext = false
    
    // Error dialog state
    @Published var showErrorAlert = false
    @Published var errorMessage = ""
    
    var formattedDuration: String {
        let minutes = Int(duration) / 60
        let seconds = Int(duration) % 60
        return String(format: "%02d:%02d", minutes, seconds)
    }
    
    private let tobogganClient: TobogganClient
    private var currentState: State?
    private let notificationHandler: NotificationHandler
    private var pendingStateUpdate: State?
    private var talkLoaded = false
    
    init() {
        // Initialize notification handler and client synchronously
        self.notificationHandler = NotificationHandler(viewModel: nil)
        self.tobogganClient = TobogganClient(
            config: TobogganConfig.shared,
            handler: self.notificationHandler
        )
        self.notificationHandler.viewModel = self
        
        // Connect asynchronously
        connectAndFetchTalk()
    }
    
    // MARK: - TobogganCore Integration
    
    private func connectAndFetchTalk() {
        
        // Connect and fetch talk info on background thread
        DispatchQueue.global(qos: .background).async { [weak self] in
            guard let self = self else { return }
            
            // Update connection status
            Task { @MainActor in
                self.connectionStatus = .connecting
            }
            
            // Connect to server
            self.tobogganClient.connect()
            
            self.fetchTalkInfoDirect(client: self.tobogganClient)
        }
    }
    
    private func fetchTalkInfoDirect(client: TobogganClient) {
        
        if let talk = client.getTalk() {
            
            // Note: talk.slides contains slide IDs (which happen to be titles in current implementation)
            // We don't store them - we'll fetch slides on demand using client.getSlide()
            
            Task { @MainActor in
                // Update presentation title
                presentationTitle = "\(talk.title) - \(talk.date)"
                
                // Store the count of slides
                totalSlides = talk.slides.count
                
                // Mark talk as loaded
                talkLoaded = true
                
                // Process any pending state updates
                if let pendingState = pendingStateUpdate {
                    handleStateChange(pendingState)
                    pendingStateUpdate = nil
                }
            }
        } else {
            Task { @MainActor in
                handleError("Could not fetch talk information from server")
            }
        }
    }
    
    // MARK: - Notification Handlers
    
    func handleStateChange(_ state: State) {
        
        Task { @MainActor in
            currentState = state
            
            switch state {
            case .`init`(let next):
                // In init state, we don't have a current slide yet
                updatePresentationState(currentSlideId: nil, nextSlideId: next)
            case .running(let previous, let current, let next, let totalDuration):
                updatePresentationState(
                    isPlaying: true,
                    duration: totalDuration,
                    currentSlideId: current,
                    previousSlideId: previous,
                    nextSlideId: next
                )
            case .paused(let previous, let current, let next, let totalDuration):
                updatePresentationState(
                    duration: totalDuration,
                    currentSlideId: current,
                    previousSlideId: previous,
                    nextSlideId: next
                )
            case .done(let previous, let current, let totalDuration):
                updatePresentationState(
                    duration: totalDuration,
                    currentSlideId: current,
                    previousSlideId: previous
                )
            }
        }
    }
    
    private func updatePresentationState(
        isPlaying: Bool = false,
        duration: TimeInterval = 0,
        currentSlideId: String?,
        previousSlideId: String? = nil,
        nextSlideId: String? = nil
    ) {
        self.isPlaying = isPlaying
        self.duration = duration
        self.canGoPrevious = (previousSlideId != nil)
        self.canGoNext = (nextSlideId != nil)
        
        // Update current slide (if we have one)
        if let currentId = currentSlideId {
            updateSlideFromState(slideId: currentId)
        } else {
            // In init state - no current slide yet
            currentSlide = nil
            currentSlideIndex = nil
        }
        
        // Update next slide title by fetching it on demand
        if let nextId = nextSlideId,
           let nextSlide = tobogganClient.getSlide(slideId: nextId) {
            nextSlideTitle = nextSlide.title
        } else {
            nextSlideTitle = "<End of presentation>"
        }
    }
    
    func handleConnectionStatusChange(_ status: ConnectionStatus) {
        
        Task { @MainActor in
            switch status {
            case .connecting:
                connectionStatus = .connecting
            case .connected:
                connectionStatus = .connected
            case .closed:
                connectionStatus = .closed
            case .reconnecting:
                connectionStatus = .reconnecting
            case .error:
                connectionStatus = .error
            }
        }
    }
    
    func handleError(_ error: String) {
        Task { @MainActor in
            connectionStatus = .error
            errorMessage = error
            showErrorAlert = true
        }
    }
    
    private func updateSlideFromState(slideId: String) {
        
        // Check if talk info has been loaded
        if !talkLoaded {
            pendingStateUpdate = currentState
            return
        }
        
        // Fetch the slide on demand using tobogganClient
        if let slide = tobogganClient.getSlide(slideId: slideId) {
            currentSlide = slide
            
            // Extract slide index from the current state if available
            updateSlideIndexFromState()
        } else {
            Task { @MainActor in
                handleError("Could not fetch slide with ID '\(slideId)'")
            }
        }
    }
    
    private func updateSlideIndexFromState() {
        // Calculate slide index based on navigation state
        // Since we can navigate with previous/next, we can track position
        guard let state = currentState else { return }
        
        switch state {
        case .`init`(_):
            // In init state, no current slide yet
            currentSlideIndex = nil
        case .running(let previous, _, let next, _),
             .paused(let previous, _, let next, _):
            // In the middle of presentation
            if previous == nil {
                // First slide
                currentSlideIndex = 0
            } else if next == nil {
                // Last slide
                currentSlideIndex = max(0, totalSlides - 1)
            } else {
                // Middle slides - maintain current index if we have one
                // The index will be updated by navigation commands
                if currentSlideIndex == nil {
                    // If we don't have an index yet, assume we're somewhere in the middle
                    currentSlideIndex = 1 // Default to second slide if we can't determine exact position
                }
            }
        case .done(_, _, _):
            // At the end
            currentSlideIndex = max(0, totalSlides - 1)
        }
    }
    
    
    // MARK: - Actions
    
    func nextSlide() {
        tobogganClient.sendCommand(command: .next)
        // Update local index optimistically
        if let current = currentSlideIndex, current < totalSlides - 1 {
            currentSlideIndex = current + 1
        } else if currentSlideIndex == nil {
            // If we're in init state and going next, we start at first slide
            currentSlideIndex = 0
        }
    }
    
    func previousSlide() {
        tobogganClient.sendCommand(command: .previous)
    }
    
    func firstSlide() {
        tobogganClient.sendCommand(command: .first)
    }
    
    func lastSlide() {
        tobogganClient.sendCommand(command: .last)
    }
    
    func togglePlayPause() {
        if isPlaying {
            tobogganClient.sendCommand(command: .pause)
        } else {
            tobogganClient.sendCommand(command: .resume)
        }
    }
    
    func blink() {
        tobogganClient.sendCommand(command: .blink)
    }
    
    deinit {
        // Client will be cleaned up automatically when deallocated
    }
}
