//
//  TobogganSession.swift
//  TobogganApp
//

import Foundation

/// The whole deck, read once per connection.
///
/// Every FFI read is blocking, so they happen together, off the main actor, and
/// the UI afterwards is pure index arithmetic. The previous shape fetched a
/// slide inside each state change — on the main thread, on every tap.
struct Deck: Sendable {
    var title: String
    var date: String
    var slides: [Slide]

    var titles: [String] {
        slides.map(\.title)
    }
}

/// Owns the Rust client and keeps its blocking calls off the main actor.
///
/// Marked `@unchecked Sendable` deliberately: `TobogganClient` is a handle to a
/// Rust object that is `Send + Sync` on that side, but `UniFFI` cannot express
/// that in Swift.
final class TobogganSession: @unchecked Sendable {
    private let client: TobogganClient

    init(url: String, clientName: String, handler: ClientNotificationHandler) {
        let config = ClientConfig(
            url: url,
            maxRetries: 5,
            retryDelay: 1.0
        )
        client = TobogganClient(config: config, clientName: clientName, handler: handler)
    }

    var isConnected: Bool {
        client.isConnected()
    }

    func connect() async {
        let client = client
        await Task.detached { client.connect() }.value
    }

    func send(_ command: Command) async {
        let client = client
        await Task.detached { client.sendCommand(command: command) }.value
    }

    /// Reads the talk and every slide in one hop off the main actor.
    ///
    /// Returns `nil` when the talk is not available yet — the socket can be up
    /// before the REST fetch has landed, and that is not an error.
    func loadDeck() async -> Deck? {
        let client = client
        return await Task.detached {
            guard let talk = client.getTalk() else {
                return nil
            }
            // One consistent read. Asking slide by slide across `talk.titles`
            // and dropping the misses — which is what this did — silently
            // *shortened* the deck whenever the talk and slide channels had not
            // both landed, shifting every slide after the gap and pointing
            // `goTo` at the wrong one. `getDeck` pairs them under one borrow.
            return Deck(title: talk.title, date: talk.date, slides: client.getDeck())
        }.value
    }
}
