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

    /// The role the server granted, which starts as the one that can do the
    /// least.
    ///
    /// Not an optional. Read as `role != .audience`, an unset optional made
    /// `isPresenter` true *before the server had said anything* — so the app
    /// offered controls it did not have and the user found out by pressing one.
    /// `toboggan-core` defaults the other way on purpose: a role that arrives
    /// unset is the one that can do the least.
    private(set) var role: ClientRole = .audience

    /// Whether the server has actually answered with a role yet. Distinct from
    /// the role itself, so the connection sheet can say "not registered" rather
    /// than claim this client is audience before anyone has decided.
    private(set) var isRegistered = false

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

    /// How many reveal states the current slide has, counting the slide as it
    /// first appears — or `nil` when the server has not counted them.
    ///
    /// The server counts *additional* reveals, so a slide with two of them has
    /// three states and `currentStep` runs `0...2`.
    var stepStates: Int? {
        currentSlide?.stepCount.map { Int($0) + 1 }
    }

    var isPresenter: Bool {
        role == .presenter
    }

    var isConnected: Bool {
        if case .connected = connection { return true }
        return false
    }

    var prevIntent: Intent {
        currentStep == 0 ? .slide : .step
    }

    var nextIntent: Intent {
        guard let stepCount = currentSlide?.stepCount else {
            // The server has not counted this slide's reveals. `NextStep` moves
            // on to the next slide once the current one runs out, so asking for
            // a step is right either way — while asking for a slide throws away
            // any reveals that were there.
            return .step
        }
        // `currentStep` runs `0...stepCount`, so the last step is `stepCount`
        // and not `stepCount - 1`. Off by one, the phone sent "next slide" one
        // reveal early and skipped the last build on every slide in the deck.
        return currentStep >= Int(stepCount) ? .slide : .step
    }

    var canGoPrev: Bool {
        guard isPresenter, isConnected else { return false }
        return prevIntent == .step || (currentSlideIndex ?? 0) > 0
    }

    var canGoNext: Bool {
        // Also gated on the connection, which `Blink` already was. Without it,
        // a phone that never reached the server kept both arrows live and
        // dropped every tap in silence.
        guard isPresenter, isConnected else { return false }
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

    /// The address without its query, which is where the presenter token rides.
    ///
    /// Every log line goes to `OSLog` as `.public` and into a sheet with a share
    /// button on it, so logging `clientURL` whole put the token in the unified
    /// log and one tap from AirDrop — undoing, on this side, the trouble the
    /// Rust side went to in order to make `Secret` unprintable.
    static func redactingToken(_ url: String) -> String {
        if var components = URLComponents(string: url) {
            components.query = nil
            components.fragment = nil
            if let cleaned = components.string {
                return cleaned
            }
        }
        // Unparseable, and still not allowed to carry a secret into the log.
        return url.split(separator: "?", maxSplits: 1).first.map(String.init) ?? ""
    }

    /// Connects to `url`, replacing any previous session.
    func connect(to url: String) {
        AppLog.shared.log(.connection, .info, "Connecting to \(Self.redactingToken(url))")
        connection = .connecting
        role = .audience
        isRegistered = false
        notice = nil

        let bridge = NotificationBridge(model: self)
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
            // The deck is back before its first slide, so the clock goes back
            // too — matching `startTimerIfNeeded`, which keys the timer on the
            // talk being under way rather than on this device having tapped.
            startedAt = nil
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
        isRegistered = true
        AppLog.shared.log(.connection, .info, "Registered \(clientId) as \(role)")
        // A function of the role rather than a one-way write. Set and never
        // cleared, "this client cannot present" stayed pinned above the slide
        // after the user supplied a token and was re-registered as presenter —
        // the app contradicting itself, with working buttons underneath.
        notice = role == .audience ? "Watching — this client cannot present." : nil
    }

    func handle(error kind: ErrorKind, message: String) {
        switch kind {
        case .server:
            // The server answered and declined: a permissions answer, not a
            // broken socket, and it belongs beside the controls rather than in a
            // modal that blames the network.
            AppLog.shared.log(.connection, .info, "Refused: \(message)")
            notice = message
        case .transport:
            AppLog.shared.log(.connection, .error, "Error: \(message)")
            alert = message
        }
    }

    #if DEBUG
    /// Test seam. The deck normally arrives from the Rust client, which needs a
    /// live server, so the navigation rules that depend on it — `nextIntent`,
    /// `canGoNext` — were untestable and therefore untested. Nothing in the app
    /// calls this; it lives here because `deck`'s setter is file-private.
    func setDeckForTesting(_ deck: Deck) {
        self.deck = deck
    }
    #endif

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
///
/// `@unchecked Sendable` is safe here because `model` is written exactly once,
/// in `init`, before the session that generates any callback exists — so no
/// callback thread can observe it changing. It is `weak` only to avoid a cycle
/// with the model that owns this bridge; nothing else mutates it.
final class NotificationBridge: ClientNotificationHandler, @unchecked Sendable {
    private weak var model: PresentationModel?

    init(model: PresentationModel) {
        self.model = model
    }

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

    func onError(kind: ErrorKind, error: String) {
        Task { @MainActor [model] in model?.handle(error: kind, message: error) }
    }
}
