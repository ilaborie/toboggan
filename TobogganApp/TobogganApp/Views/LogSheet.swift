//
//  LogSheet.swift
//  TobogganApp
//

import SwiftUI

/// The log, on the device.
///
/// Mirrors the terminal client's log view: a presenter debugging a phone that
/// will not reach the server is in a room, not at a debugger.
struct LogSheet: View {
    @Environment(\.dismiss)
    private var dismiss
    private var log: AppLog { AppLog.shared }

    var body: some View {
        NavigationStack {
            List(log.entries.reversed()) { entry in
                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.message)
                        .font(.footnote.monospaced())
                        .foregroundStyle(entry.level == .error ? .red : .primary)
                    Text("\(entry.date.formatted(date: .omitted, time: .standard)) · \(entry.category.rawValue)")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
            .listStyle(.plain)
            .navigationTitle("Log")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
                ToolbarItem(placement: .primaryAction) {
                    ShareLink(item: log.exportText) {
                        Image(systemName: "square.and.arrow.up")
                    }
                    .disabled(log.entries.isEmpty)
                }
                ToolbarItem(placement: .secondaryAction) {
                    Button("Clear", systemImage: "trash") {
                        log.clear()
                    }
                    .disabled(log.entries.isEmpty)
                }
            }
            .overlay {
                if log.entries.isEmpty {
                    ContentUnavailableView("No log yet", systemImage: "text.alignleft")
                }
            }
        }
    }
}
