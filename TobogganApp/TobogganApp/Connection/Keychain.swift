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

    /// Reads the stored value, or `nil` when there is nothing to read.
    ///
    /// A read that *failed* is logged before it returns `nil`. Folded into the
    /// same silent `nil` as "nothing stored", a locked keychain looked exactly
    /// like a device that had never been configured: the app dropped to
    /// audience, every control went dead, and nothing anywhere said why.
    static func string(for account: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)

        switch status {
        case errSecSuccess:
            guard let data = item as? Data, let value = String(data: data, encoding: .utf8) else {
                AppLog.shared.log(.connection, .error, "The stored token is unreadable and will be ignored")
                return nil
            }
            return value
        case errSecItemNotFound:
            // Nothing scanned yet. The ordinary case, and not worth a line.
            return nil
        default:
            AppLog.shared.log(
                .connection,
                .error,
                "Keychain read failed (OSStatus \(status)); this device will register as audience"
            )
            return nil
        }
    }

    /// Stores `value`, or removes the entry when it is `nil` or empty — so
    /// clearing the field in the UI actually clears the secret.
    ///
    /// Returns whether the keychain now holds what was asked of it. The delete
    /// runs first, which keeps the add-versus-update branch out of this
    /// function but also means a failed add is *destructive*: the previous
    /// token is already gone. Discarding the result left the in-memory value
    /// showing as saved and the next launch registering as audience.
    @discardableResult
    static func set(_ value: String?, for account: String) -> Bool {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
        let deleted = SecItemDelete(query as CFDictionary)
        guard deleted == errSecSuccess || deleted == errSecItemNotFound else {
            AppLog.shared.log(.connection, .error, "Keychain delete failed (OSStatus \(deleted))")
            return false
        }

        guard let value, !value.isEmpty, let data = value.data(using: .utf8) else {
            // Clearing was the request, and the delete above did it.
            return true
        }
        var insert = query
        insert[kSecValueData as String] = data
        // The token is only needed while the presenter is using the app, and
        // this keeps it off backups and off other devices.
        insert[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly

        let added = SecItemAdd(insert as CFDictionary, nil)
        guard added == errSecSuccess else {
            AppLog.shared.log(
                .connection,
                .error,
                "Keychain write failed (OSStatus \(added)); the token will be forgotten when the app closes"
            )
            return false
        }
        return true
    }
}
