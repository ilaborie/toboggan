//
//  Keychain.swift
//  TobogganApp
//

import Foundation
import Security

/// The smallest keychain wrapper that will hold one presenter token.
///
/// The token lives here rather than in `UserDefaults` because the Rust side went
/// to some trouble to make it unloggable — `Secret` has a hand-written `Debug`
/// so no format string can print it — and a plist in the app container would
/// undo that for the sake of four fewer lines.
enum Keychain {
    private static let service = "dev.toboggan.TobogganApp"

    static func string(for account: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess,
              let data = item as? Data,
              let value = String(data: data, encoding: .utf8)
        else {
            return nil
        }
        return value
    }

    /// Stores `value`, or removes the entry when it is `nil` or empty — so
    /// clearing the field in the UI actually clears the secret.
    static func set(_ value: String?, for account: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
        SecItemDelete(query as CFDictionary)

        guard let value, !value.isEmpty, let data = value.data(using: .utf8) else {
            return
        }
        var insert = query
        insert[kSecValueData as String] = data
        // The token is only needed while the presenter is using the app, and
        // this keeps it off backups and off other devices.
        insert[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        SecItemAdd(insert as CFDictionary, nil)
    }
}
