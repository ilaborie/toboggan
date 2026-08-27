//
//  PresentationModel.swift
//  TobogganApp
//

import Foundation

/// The state of the talk as this device sees it.
///
/// The server is authoritative for everything here: nothing is updated
/// optimistically. The old code bumped the slide index locally on "next" and not
/// on "previous", which was already inconsistent — and once the server started
/// refusing commands from clients that are not the presenter, an optimistic
/// update meant the phone moved and the projector did not.
@MainActor
@Observable
final class PresentationModel {
    /// What the prev/next buttons mean right now.
    ///
    /// One value replaces the four mutually exclusive buttons and six booleans
    /// the old controls needed: on the first step "previous" means the previous
    /// slide, otherwise the previous step.
    enum Intent {
        case step
        case slide
    }

    private(set) var deck = Deck(title: "", date: "", slides: [])
    private(set) var connection: ConnectionStatus = .closed
    private(set) var role: ClientRole?
    private(set) var currentSlideIndex: Int?
    private(set) var currentStep = 0

    /// A refusal or other protocol-level complaint. Shown inline: it is not a
    /// connection failure, and the old code raised it as a modal "Connection
    /// Error" alert, which blamed the network for a permissions decision.
    var notice: String?

    /// A genuine transport failure, worth interrupting for.
    var alert: String?

    private var session: TobogganSession?
    private var bridge: NotificationBridge?

    // MARK: - Derived state

    var totalSlides: Int {
        deck.slides.count
    }

    var currentSlide: Slide? {
        currentSlideIndex.flatMap { deck.slides.indices.contains($0) ? deck.slides[$0] : nil }
    }

    var nextSlide: Slide? {
        // Before the talk starts there is no current slide, and what comes next
        // is the first one — not the end of the deck.
        guard let index = currentSlideIndex else {
            return deck.slides.first
        }
        guard deck.slides.indices.contains(index + 1) else {
            return nil
        }
        return deck.slides[index + 1]
    }

    var stepCount: Int {
        Int(currentSlide?.stepCount ?? 0)
    }

    var isPresenter: Bool {
        role != .audience
    }

    var isConnected: Bool {
        if case .connected = connection { return true }
        return false
    }

    var prevIntent: Intent {
        currentStep == 0 ? .slide : .step
    }

    var nextIntent: Intent {
        (stepCount == 0 || currentStep >= stepCount - 1) ? .slide : .step
    }

    var canGoPrev: Bool {
        guard isPresenter else { return false }
        return prevIntent == .step || (currentSlideIndex ?? 0) > 0
    }

    var canGoNext: Bool {
        guard isPresenter else { return false }
        guard let index = currentSlideIndex else { return totalSlides > 0 }
        return nextIntent == .step || index + 1 < totalSlides
    }

    // MARK: - Pacing

    /// When the talk started, or `nil` before the first command.
    private(set) var startedAt: Date?

    /// Seconds since the talk started, as of `date`.
    ///
    /// Takes the date rather than reading the clock so the caller can drive it
    /// from a `TimelineView` — a computed property reading `Date()` would only
    /// be re-evaluated when something else invalidated the view, which for a
    /// clock means never.
    func elapsed(at date: Date) -> TimeInterval? {
        startedAt.map { date.timeIntervalSince($0) }
    }

    /// Whether the deck plans any timings at all. When it does not, the pacing
    /// readout is hidden rather than shown as zero — the same rule the web
    /// presenter view uses.
    var hasPlannedTimings: Bool {
        deck.slides.contains { $0.durationSecs != nil }
    }

    /// How far ahead (negative) or behind (positive) the plan we are, in seconds.
    ///
    /// The plan for "now" is the sum of the durations of the slides already left
    /// behind, which is how the web presenter computes it.
    func pacingDrift(at date: Date) -> TimeInterval? {
        guard hasPlannedTimings, let elapsed = elapsed(at: date), let index = currentSlideIndex
        else {
            return nil
        }
        let planned = deck.slides.prefix(index).reduce(into: 0.0) { total, slide in
            total += TimeInterval(slide.durationSecs ?? 0)
        }
        return elapsed - planned
    }

    func restartTimer() {
        startedAt = Date()
    }

    /// Starts the clock the first time the deck is under way.
    ///
    /// Keyed on the talk running rather than on this device sending a command:
    /// a phone held as a second screen while someone drives from a laptop is
    /// still in the same talk, and its timer should say so.
    private func startTimerIfNeeded() {
        guard startedAt == nil else { return }
        startedAt = Date()
    }

    // MARK: - Lifecycle

    /// Connects to `url`, replacing any previous session.
    func connect(to url: String) {
        AppLog.shared.log(.connection, .info, "Connecting to \(url)")
        connection = .connecting
        role = nil
        notice = nil

        let bridge = NotificationBridge()
        bridge.model = self
        let session = TobogganSession(url: url, clientName: "TobogganApp", handler: bridge)
        self.bridge = bridge
        self.session = session

        Task {
            await session.connect()
            await reloadDeck()
        }
    }

    func send(_ command: Command) {
        guard let session else {
            AppLog.shared.log(.ffi, .error, "Command \(command) dropped: no session")
            return
        }
        AppLog.shared.log(.ffi, .debug, "Sending \(command)")
        Task { await session.send(command) }
    }

    func goPrev() {
        send(prevIntent == .step ? .previousStep : .previous)
    }

    func goNext() {
        send(nextIntent == .step ? .nextStep : .next)
    }

    func goTo(slide index: Int) {
        send(.goTo(slide: UInt32(index)))
    }

    // MARK: - Notification handling

    func handle(state: PresentationState) {
        switch state {
        case let .`init`(totalSlides):
            AppLog.shared.log(.ui, .debug, "State: init, \(totalSlides) slides")
            currentSlideIndex = nil
            currentStep = 0
        case let .running(_, current, _, step, _):
            startTimerIfNeeded()
            currentSlideIndex = Int(current)
            currentStep = Int(step)
        case let .done(_, current, step, _):
            startTimerIfNeeded()
            currentSlideIndex = Int(current)
            currentStep = Int(step)
        }
    }

    func handle(connection status: ConnectionStatus) {
        connection = status
        switch status {
        case .connected:
            AppLog.shared.log(.connection, .info, "Connected")
        case let .reconnecting(attempt, maxAttempt, delaySecs):
            // The detail this line carries is the reason the FFI stopped
            // flattening the status: "Reconnecting..." on its own tells someone
            // standing in a room nothing they can act on.
            AppLog.shared.log(
                .connection,
                .info,
                "Reconnecting, attempt \(attempt)/\(maxAttempt) in \(delaySecs)s"
            )
        case let .error(message):
            AppLog.shared.log(.connection, .error, "Connection error: \(message)")
            alert = message
        case .connecting:
            AppLog.shared.log(.connection, .debug, "Connecting")
        case .closed:
            AppLog.shared.log(.connection, .info, "Closed")
        }
    }

    func handle(registered clientId: String, role: ClientRole) {
        self.role = role
        AppLog.shared.log(.connection, .info, "Registered \(clientId) as \(role)")
        if role == .audience {
            notice = "Watching — this client cannot present."
        }
    }

    func handle(error message: String) {
        // A refused command is a permissions answer, not a broken socket. The
        // server says so in as many words, and the app should not translate that
        // into a network alert.
        if role == .audience || message.localizedCaseInsensitiveContains("watching") {
            AppLog.shared.log(.connection, .info, "Refused: \(message)")
            notice = message
        } else {
            AppLog.shared.log(.connection, .error, "Error: \(message)")
            alert = message
        }
    }

    func reloadDeck() async {
        guard let session else { return }
        guard let loaded = await session.loadDeck() else {
            AppLog.shared.log(.ffi, .error, "Talk not available yet")
            return
        }
        deck = loaded
        AppLog.shared.log(.ffi, .info, "Loaded \(loaded.slides.count) slides for \(loaded.title)")
    }
}

/// Bridges the Rust callbacks, which arrive on their own threads, onto the main
/// actor where the model lives.
final class NotificationBridge: ClientNotificationHandler, @unchecked Sendable {
    weak var model: PresentationModel?

    func onStateChange(state: PresentationState) {
        Task { @MainActor [model] in model?.handle(state: state) }
    }

    func onTalkChange(state: PresentationState) {
        Task { @MainActor [model] in
            await model?.reloadDeck()
            model?.handle(state: state)
        }
    }

    func onConnectionStatusChange(status: ConnectionStatus) {
        Task { @MainActor [model] in model?.handle(connection: status) }
    }

    func onRegistered(clientId: String, role: ClientRole) {
        Task { @MainActor [model] in model?.handle(registered: clientId, role: role) }
    }

    func onClientConnected(clientId: String, name: String) {
        AppLog.shared.log(.connection, .debug, "Client joined: \(name)")
    }

    func onClientDisconnected(clientId: String, name: String) {
        AppLog.shared.log(.connection, .debug, "Client left: \(name)")
    }

    func onError(error: String) {
        Task { @MainActor [model] in model?.handle(error: error) }
    }
}
