//
//  AppLog.swift
//  TobogganApp
//

import Foundation
import OSLog

/// Logging for a device that is usually not attached to Xcode.
///
/// Someone debugging a phone that will not reach the server is standing in a
/// room, not sitting at a debugger, so every line goes two places: `OSLog` for
/// the console, and an in-memory ring the app can show in a sheet. The terminal
/// client has had the same log view behind `l` for as long as it has existed.
@MainActor
@Observable
final class AppLog {
    static let shared = AppLog()

    /// What a line is about. Enough to filter by in Console.app without
    /// inventing a category per call site.
    enum Category: String, CaseIterable, Sendable {
        case connection
        case ffi
        case ui
    }

    enum Level: String, Sendable {
        case debug
        case info
        case error
    }

    struct Entry: Identifiable, Sendable {
        let id = UUID()
        let date: Date
        let category: Category
        let level: Level
        let message: String
    }

    /// Old lines are dropped rather than grown without bound — this exists to be
    /// read during a talk, and nobody scrolls back further than this.
    private static let capacity = 400

    private(set) var entries: [Entry] = []

    private init() {}

    /// Records a line. Safe to call from the Rust callback threads, which is
    /// where the interesting ones come from.
    nonisolated func log(_ category: Category, _ level: Level, _ message: String) {
        let logger = Logger(subsystem: "dev.toboggan.TobogganApp", category: category.rawValue)
        switch level {
        case .debug: logger.debug("\(message, privacy: .public)")
        case .info: logger.info("\(message, privacy: .public)")
        case .error: logger.error("\(message, privacy: .public)")
        }
        let entry = Entry(date: Date(), category: category, level: level, message: message)
        Task { @MainActor in
            AppLog.shared.append(entry)
        }
    }

    func clear() {
        entries.removeAll()
    }

    /// The whole ring as text, for the share sheet — the only way a log gets off
    /// a device that is not plugged into anything.
    var exportText: String {
        let formatter = ISO8601DateFormatter()
        return entries
            .map { "\(formatter.string(from: $0.date)) [\($0.category.rawValue)] \($0.message)" }
            .joined(separator: "\n")
    }

    private func append(_ entry: Entry) {
        entries.append(entry)
        if entries.count > Self.capacity {
            entries.removeFirst(entries.count - Self.capacity)
        }
    }
}
