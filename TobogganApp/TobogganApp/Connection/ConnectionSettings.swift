//
//  ConnectionSettings.swift
//  TobogganApp
//

import Foundation

/// Where the presenter token is kept.
///
/// A protocol so the tests can run without the device keychain. Constructing
/// `ConnectionSettings` used to read and write the real one, which left a token
/// on the simulator between runs and made the suite depend on the order its
/// cases happened to execute in.
protocol TokenStore: Sendable {
    func read(_ account: String) -> String?
    /// Whether the store now holds what was asked of it.
    @discardableResult
    func write(_ value: String?, for account: String) -> Bool
}

struct KeychainTokenStore: TokenStore {
    func read(_ account: String) -> String? {
        Keychain.string(for: account)
    }

    func write(_ value: String?, for account: String) -> Bool {
        Keychain.set(value, for: account)
    }
}

/// Where the server is, and what proves we may drive it.
///
/// Kept apart from the presentation state because it outlives any one
/// connection: it is what the app reads at launch and what the connection sheet
/// writes. The URL is ordinary preference data; the token is not, so it lives in
/// the keychain (see ``Keychain``).
@MainActor
@Observable
final class ConnectionSettings {
    private enum Key {
        static let serverURL = "serverURL"
        static let token = "presenterToken"
    }

    /// Loopback is right for the simulator, which shares the Mac's network, and
    /// wrong for every real device — which is the whole reason the connection
    /// sheet exists.
    static let defaultServerURL = "http://127.0.0.1:8080"

    @ObservationIgnored private let defaults: UserDefaults
    @ObservationIgnored private let tokens: TokenStore

    var serverURL: String {
        didSet { defaults.set(serverURL, forKey: Key.serverURL) }
    }

    var presenterToken: String {
        didSet {
            storageWarning = tokens.write(presenterToken, for: Key.token)
                ? nil
                : "The token could not be saved. It works now and will be forgotten when the app closes."
        }
    }

    /// Set when the keychain refused the token.
    ///
    /// Worth saying out loud: the in-memory value still works for this session,
    /// so without this the app looks configured and comes back as audience after
    /// the next launch with nothing to explain it.
    private(set) var storageWarning: String?

    init(defaults: UserDefaults = .standard, tokens: TokenStore = KeychainTokenStore()) {
        self.defaults = defaults
        self.tokens = tokens
        serverURL = defaults.string(forKey: Key.serverURL) ?? Self.defaultServerURL
        presenterToken = tokens.read(Key.token) ?? ""
    }

    /// Whether the address alone can reach anything. A phone on `127.0.0.1` is
    /// talking to itself, which is worth saying out loud rather than letting it
    /// fail as a timeout.
    var isLoopback: Bool {
        guard let host = URLComponents(string: serverURL)?.host else {
            return false
        }
        return host == "127.0.0.1" || host == "localhost" || host == "::1"
    }

    /// Why this address cannot be used, or `nil` when it can.
    ///
    /// The scanned path validated and the typed path did not, so `hello` was
    /// storable, kept, and handed to the client — where it surfaced as
    /// `relative URL without a base` in a modal, thirty seconds before a talk.
    /// Both doors now go through the same check.
    var addressProblem: String? {
        let trimmed = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            return "Enter the address the server is listening on."
        }
        guard let components = URLComponents(string: trimmed), let scheme = components.scheme else {
            return "The address needs a scheme — for example http://192.168.1.10:8080"
        }
        guard scheme == "http" || scheme == "https" else {
            return "Toboggan speaks http and https; “\(scheme)://” will not connect."
        }
        guard let host = components.host, !host.isEmpty else {
            return "The address is missing a host name."
        }
        return nil
    }

    /// The single string the Rust client wants: address plus `?token=…`.
    ///
    /// `split_presenter_token` on the other side parses the token back out, so
    /// composing it here keeps the FFI surface to one field and means the token
    /// travels the same way whether it was typed or scanned.
    ///
    /// Encoded against `.alphanumerics`, which is deliberately far stricter than
    /// necessary: it leaves no `+` and no space for the server's form-decoding
    /// to read as something else.
    var clientURL: String {
        let base = serverURL.trimmingCharacters(in: .whitespacesAndNewlines)
        let token = presenterToken.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !token.isEmpty else {
            return base
        }
        var components = URLComponents(string: base)
        let encoded = token.addingPercentEncoding(withAllowedCharacters: .alphanumerics) ?? token
        components?.percentEncodedQuery = "token=\(encoded)"
        return components?.string ?? "\(base)?token=\(encoded)"
    }

    /// Takes apart a scanned URL. The server's homepage link carries the token in
    /// its query, so one scan configures both fields.
    func apply(scanned text: String) -> Bool {
        guard var components = URLComponents(string: text.trimmingCharacters(in: .whitespacesAndNewlines)),
              let scheme = components.scheme,
              scheme == "http" || scheme == "https",
              components.host != nil
        else {
            return false
        }
        let scannedToken = components.percentEncodedQuery.flatMap(Self.formDecodedToken(in:))
        components.query = nil
        components.fragment = nil
        // The server prints links to `/run` and `/presenter`, and the QR payload
        // is a bare `http://<host>/?token=…`. The client wants the origin either
        // way; it appends its own API paths, and a kept path would double up.
        components.path = ""

        serverURL = components.string ?? text
        if let scannedToken, !scannedToken.isEmpty {
            presenterToken = scannedToken
        }
        return true
    }

    /// Reads `token` out of a raw query string the way the server reads it.
    ///
    /// `URLComponents.queryItems` percent-decodes but leaves `+` alone, while
    /// `Secret::from_query_value` form-decodes it to a space. A token containing
    /// a space therefore came back as a *different* secret than the one the
    /// server holds, and the phone registered as audience with every button
    /// enabled and nothing to say why.
    private static func formDecodedToken(in query: String) -> String? {
        for pair in query.split(separator: "&") where pair.hasPrefix("token=") {
            let raw = String(pair.dropFirst("token=".count))
            let spaced = raw.replacingOccurrences(of: "+", with: " ")
            return spaced.removingPercentEncoding ?? spaced
        }
        return nil
    }
}
