//
//  TobogganAppTests.swift
//  TobogganAppTests
//

import Foundation
import Testing
@testable import TobogganApp

struct TobogganAppTests {
    @Test
    func uniffiInitialisesWithoutChecksumMismatch() async {
        let config = ClientConfig(url: "http://localhost:8080", maxRetries: 3, retryDelay: 1.0)
        let client = TobogganClient(
            config: config,
            clientName: "iOS Test",
            handler: TestNotificationHandler()
        )

        // This used to be unsatisfiable: `is_connected` returned `true`
        // unconditionally on the Rust side, so a client that had never opened a
        // socket claimed to be connected.
        #expect(client.isConnected() == false)
    }

    /// The FFI enum has to *keep* covering the protocol. Listing the cases and
    /// counting the list proved nothing — a ninth command would not have failed
    /// it. Switching exhaustively does: a new case stops compiling here.
    @Test
    func everyCommandHasAMeaning() async {
        for command in Command.allProtocolCases {
            let described: String =
                switch command {
                case .next: "next slide"
                case .previous: "previous slide"
                case .first: "first slide"
                case .last: "last slide"
                case .nextStep: "next step"
                case .previousStep: "previous step"
                case .blink: "blink"
                case .goTo: "jump to a slide"
                }
            #expect(!described.isEmpty)
        }
        #expect(Command.allProtocolCases.contains(.goTo(slide: 3)))
    }
}

extension Command {
    /// Every command the app can send. Kept beside the exhaustive switch above,
    /// which is what actually fails when the FFI grows a case.
    static let allProtocolCases: [Command] = [
        .next, .previous, .first, .last, .nextStep, .previousStep, .blink, .goTo(slide: 3)
    ]
}

// MARK: - ConnectionSettings

/// Holds a token in memory, so the suite never touches the device keychain.
final class InMemoryTokenStore: TokenStore, @unchecked Sendable {
    private var stored: [String: String] = [:]
    /// Set to make `write` fail, which is the path that used to be silent.
    var refuseWrites = false

    init(initial: [String: String] = [:]) {
        stored = initial
    }

    func read(_ account: String) -> String? {
        stored[account]
    }

    func write(_ value: String?, for account: String) -> Bool {
        guard !refuseWrites else { return false }
        if let value, !value.isEmpty {
            stored[account] = value
        } else {
            stored.removeValue(forKey: account)
        }
        return true
    }
}

@MainActor
struct ConnectionSettingsTests {
    /// A fresh, isolated pair of stores per test. Sharing `UserDefaults.standard`
    /// and the real keychain made these order-dependent and leaked a token onto
    /// the simulator.
    private func makeSettings(
        tokens: InMemoryTokenStore = InMemoryTokenStore()
    ) -> ConnectionSettings {
        let suite = UserDefaults(suiteName: "dev.toboggan.tests.\(UUID().uuidString)")
        return ConnectionSettings(defaults: suite ?? .standard, tokens: tokens)
    }

    @Test
    func aTokenIsAppendedAsAQueryParameter() {
        let settings = makeSettings()
        settings.serverURL = "http://192.168.1.10:8080"
        settings.presenterToken = "s3cr3t"
        #expect(settings.clientURL == "http://192.168.1.10:8080?token=s3cr3t")
    }

    @Test
    func noTokenLeavesTheAddressAlone() {
        let settings = makeSettings()
        settings.serverURL = "http://192.168.1.10:8080"
        settings.presenterToken = ""
        #expect(settings.clientURL == "http://192.168.1.10:8080")
    }

    /// A token with a space or a `+` has to survive the round trip; the server
    /// and the web client disagreed about this once already.
    @Test
    func anAwkwardTokenIsPercentEncoded() {
        let settings = makeSettings()
        settings.serverURL = "http://192.168.1.10:8080"
        settings.presenterToken = "a b+c"
        #expect(settings.clientURL == "http://192.168.1.10:8080?token=a%20b%2Bc")
    }

    /// One scan has to configure both fields: the server's link carries the
    /// token in its query.
    @Test
    func scanningALinkSetsTheAddressAndTheToken() {
        let settings = makeSettings()
        #expect(settings.apply(scanned: "http://192.168.1.10:8080/run?token=s3cr3t"))
        #expect(settings.serverURL == "http://192.168.1.10:8080")
        #expect(settings.presenterToken == "s3cr3t")
    }

    /// The QR the server prints is a bare origin with a trailing slash. The path
    /// has to go either way, or the client's own `/api/…` doubles up.
    @Test
    func scanningTheHomepageQrKeepsOnlyTheOrigin() {
        let settings = makeSettings()
        #expect(settings.apply(scanned: "http://192.168.1.10:8080/?token=s3cr3t"))
        #expect(settings.serverURL == "http://192.168.1.10:8080")
    }

    /// `+` is a space in a query string. `URLComponents.queryItems` does not
    /// know that and the server's `Secret::from_query_value` does, so a scanned
    /// `a+b` used to become the token `a+b` here and `a b` there — a silent
    /// demotion to audience with every button still enabled.
    @Test
    func aScannedTokenIsFormDecodedTheWayTheServerDecodesIt() {
        let settings = makeSettings()
        #expect(settings.apply(scanned: "http://192.168.1.10:8080/?token=a+b"))
        #expect(settings.presenterToken == "a b")

        #expect(settings.apply(scanned: "http://192.168.1.10:8080/?token=a%20b%2Bc"))
        #expect(settings.presenterToken == "a b+c")
    }

    @Test
    func scanningSomethingElseIsRejected() {
        let settings = makeSettings()
        #expect(settings.apply(scanned: "WIFI:S:somenetwork;T:WPA;") == false)
    }

    @Test
    func loopbackIsRecognised() {
        let settings = makeSettings()
        settings.serverURL = "http://127.0.0.1:8080"
        #expect(settings.isLoopback)
        settings.serverURL = "http://192.168.1.10:8080"
        #expect(settings.isLoopback == false)
    }

    /// The typed path used to accept anything the scanned path would refuse, so
    /// `hello` reached the client and came back as a URL-parser message.
    @Test
    func atypedAddressIsHeldToTheSameRuleAsAScannedOne() {
        let settings = makeSettings()
        settings.serverURL = "http://192.168.1.10:8080"
        #expect(settings.addressProblem == nil)

        settings.serverURL = "hello"
        #expect(settings.addressProblem != nil)

        settings.serverURL = "192.168.1.10:8080"
        #expect(settings.addressProblem != nil, "a bare host:port has no usable scheme")

        settings.serverURL = "ftp://192.168.1.10:8080"
        #expect(settings.addressProblem != nil)

        settings.serverURL = ""
        #expect(settings.addressProblem != nil)
    }

    /// A keychain that refuses the write leaves the app holding a token that
    /// will not survive a relaunch. Discarded, that surfaced only as "watching,
    /// not presenting" on the next launch, with nothing to explain it.
    @Test
    func aRefusedTokenWriteIsReported() {
        let store = InMemoryTokenStore()
        let settings = makeSettings(tokens: store)
        settings.presenterToken = "s3cr3t"
        #expect(settings.storageWarning == nil)

        store.refuseWrites = true
        settings.presenterToken = "another"
        #expect(settings.storageWarning != nil)
    }
}

final class TestNotificationHandler: ClientNotificationHandler, @unchecked Sendable {
    func onStateChange(state: PresentationState) {}
    func onTalkChange(state: PresentationState) {}
    func onConnectionStatusChange(status: ConnectionStatus) {}
    func onError(kind: ErrorKind, error: String) {}
    func onRegistered(clientId: String, role: ClientRole) {}
    func onClientConnected(clientId: String, name: String) {}
    func onClientDisconnected(clientId: String, name: String) {}
}
