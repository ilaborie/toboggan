//
//  PacingBadge.swift
//  TobogganApp
//

import SwiftUI

/// Elapsed time, and how it compares with the plan.
///
/// Hidden entirely when the deck plans no timings — the same rule the web
/// presenter view follows, and the reason the old duration badge was worse than
/// nothing: it was wired to a value that was never set, so it read `00:00` for
/// the whole talk.
struct PacingBadge: View {
    let elapsed: TimeInterval
    let drift: TimeInterval?

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: "clock")
                .font(.caption2)
            Text(Self.clock(elapsed))
                .font(.caption.monospacedDigit())
            if let drift, abs(drift) >= 30 {
                Text(Self.driftText(drift))
                    .font(.caption.monospacedDigit().weight(.semibold))
                    .foregroundStyle(drift > 0 ? .orange : .green)
            }
        }
        .foregroundStyle(.secondary)
        .accessibilityElement(children: .combine)
    }

    /// `mm:ss`, matching the format the rest of the project prints durations in.
    static func clock(_ interval: TimeInterval) -> String {
        let total = Int(interval.rounded(.down))
        return String(format: "%02d:%02d", total / 60, total % 60)
    }

    /// Ahead reads as a negative drift, behind as positive — the sign the web
    /// presenter uses, shown here with a word so it needs no decoding.
    static func driftText(_ drift: TimeInterval) -> String {
        let magnitude = clock(abs(drift))
        return drift > 0 ? "+\(magnitude) behind" : "−\(magnitude) ahead"
    }
}
