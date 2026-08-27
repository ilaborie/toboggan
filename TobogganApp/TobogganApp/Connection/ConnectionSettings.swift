//
//  ConnectionSettings.swift
//  TobogganApp
//

import Foundation

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

    var serverURL: String {
        didSet { UserDefaults.standard.set(serverURL, forKey: Key.serverURL) }
    }

    var presenterToken: String {
        didSet { Keychain.set(presenterToken, for: Key.token) }
    }

    init() {
        serverURL = UserDefaults.standard.string(forKey: Key.serverURL) ?? Self.defaultServerURL
        presenterToken = Keychain.string(for: Key.token) ?? ""
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

    /// The single string the Rust client wants: address plus `?token=…`.
    ///
    /// `split_presenter_token` on the other side parses the token back out, so
    /// composing it here keeps the FFI surface to one field and means the token
    /// travels the same way whether it was typed or scanned.
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
        let scannedToken = components.queryItems?
            .first { $0.name == "token" }?
            .value
        components.query = nil
        components.fragment = nil
        // The server prints links to `/run` and `/presenter`; the client wants
        // the origin, and appends its own API paths.
        components.path = ""

        serverURL = components.string ?? text
        if let scannedToken, !scannedToken.isEmpty {
            presenterToken = scannedToken
        }
        return true
    }
}
