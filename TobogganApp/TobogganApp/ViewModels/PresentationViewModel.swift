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

    func onTalkChange(state: State) {
        viewModel?.handleTalkChange(state)
    }

    func onConnectionStatusChange(status: ConnectionStatus) {
        viewModel?.handleConnectionStatusChange(status)
    }

    func onError(error: String) {
        viewModel?.handleError(error)
    }
}

// Shared configuration for TobogganClient
enum TobogganConfig {
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
    @Published var currentSlideIndex: Int?
    @Published var totalSlides: Int = 0
    @Published var currentSlide: Slide?
    @Published var duration: TimeInterval = 0
    @Published var canGoPrevious = false
    @Published var canGoNext = false

    // Step tracking
    @Published var currentStep: Int = 0
    @Published var stepCount: Int = 0

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
            case .`init`(let totalSlides):
                // In init state, we don't have a current slide yet
                self.totalSlides = Int(totalSlides)
                self.currentStep = 0
                self.stepCount = 0
                updatePresentationState(currentSlideIndex: nil)
            case let .running(previous, current, next, currentStep, stepCount, totalDuration):
                self.currentStep = Int(currentStep)
                self.stepCount = Int(stepCount)
                updatePresentationState(
                    currentSlideIndex: Int(current),
                    isPlaying: true,
                    duration: totalDuration,
                    previousSlideIndex: previous.map(Int.init),
                    nextSlideIndex: next.map(Int.init)
                )
            case let .paused(previous, current, next, currentStep, stepCount, totalDuration):
                self.currentStep = Int(currentStep)
                self.stepCount = Int(stepCount)
                updatePresentationState(
                    currentSlideIndex: Int(current),
                    duration: totalDuration,
                    previousSlideIndex: previous.map(Int.init),
                    nextSlideIndex: next.map(Int.init)
                )
            case let .done(previous, current, currentStep, stepCount, totalDuration):
                self.currentStep = Int(currentStep)
                self.stepCount = Int(stepCount)
                updatePresentationState(
                    currentSlideIndex: Int(current),
                    duration: totalDuration,
                    previousSlideIndex: previous.map(Int.init)
                )
            }
        }
    }

    func handleTalkChange(_ state: State) {
        print("📝 Presentation updated - refetching talk metadata and slides")

        // Refetch talk information in background
        DispatchQueue.global(qos: .background).async { [weak self] in
            guard let self = self else { return }
            self.fetchTalkInfoDirect(client: self.tobogganClient)
        }

        // Update state to reflect new slide position (server already adjusted)
        handleStateChange(state)
    }
    
    private func updatePresentationState(
        currentSlideIndex: Int?,
        isPlaying: Bool = false,
        duration: TimeInterval = 0,
        previousSlideIndex: Int? = nil,
        nextSlideIndex: Int? = nil
    ) {
        self.isPlaying = isPlaying
        self.duration = duration
        self.canGoPrevious = (previousSlideIndex != nil)
        self.canGoNext = (nextSlideIndex != nil)
        
        // Update current slide (if we have one)
        if let currentIdx = currentSlideIndex {
            updateSlideFromState(slideIndex: currentIdx)
        } else {
            // In init state - no current slide yet
            currentSlide = nil
            self.currentSlideIndex = nil
        }
        
        // Update next slide title by fetching it on demand
        if let nextIdx = nextSlideIndex,
           let nextSlide = tobogganClient.getSlide(index: UInt32(nextIdx)) {
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
    
    private func updateSlideFromState(slideIndex: Int) {
        // Check if talk info has been loaded
        if !talkLoaded {
            pendingStateUpdate = currentState
            return
        }
        
        // Fetch the slide on demand using tobogganClient
        if let slide = tobogganClient.getSlide(index: UInt32(slideIndex)) {
            currentSlide = slide
            
            // Set the slide index directly
            self.currentSlideIndex = slideIndex
        } else {
            Task { @MainActor in
                handleError("Could not fetch slide with index '\(slideIndex)'")
            }
        }
    }
    
    // MARK: - Actions
    
    // MARK: - Step Navigation

    func nextStep() {
        tobogganClient.sendCommand(command: .nextStep)
    }

    func previousStep() {
        tobogganClient.sendCommand(command: .previousStep)
    }

    // MARK: - Slide Navigation

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
