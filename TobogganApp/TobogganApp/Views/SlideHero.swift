//
//  SlideHero.swift
//  TobogganApp
//

import SwiftUI

/// The current slide, as content rather than as chrome.
///
/// Deliberately not glass. This is what the floating controls refract, and glass
/// over glass is the mistake the previous layout made three levels deep.
struct SlideHero: View {
    @Environment(PresentationModel.self)
    private var model

    private var title: String {
        model.currentSlide?.title ?? "Ready to start"
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 8) {
                if let kind = model.currentSlide?.kind, kind != .standard {
                    Text(kind == .cover ? "Cover" : "Part")
                        .font(.caption2.weight(.semibold))
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(.tint.opacity(0.15), in: .capsule)
                }
                Text(positionText)
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)

                if model.hasPlannedTimings, model.startedAt != nil {
                    Spacer(minLength: 8)
                    // Driven by the timeline so the clock actually advances.
                    // Tappable to restart, as the web presenter's is.
                    Button {
                        model.restartTimer()
                    } label: {
                        TimelineView(.periodic(from: .now, by: 1)) { context in
                            PacingBadge(
                                elapsed: model.elapsed(at: context.date) ?? 0,
                                drift: model.pacingDrift(at: context.date)
                            )
                        }
                    }
                    .buttonStyle(.plain)
                    .accessibilityHint("Restart the timer")
                }
            }

            Text(title)
                // A text style rather than a fixed 34pt, so it scales with
                // Dynamic Type instead of ignoring it.
                .font(.system(.largeTitle, design: .rounded, weight: .bold))
                .foregroundStyle(model.currentSlide == nil ? .secondary : .primary)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var positionText: String {
        guard let index = model.currentSlideIndex else {
            return "\(model.totalSlides) slides"
        }
        return "Slide \(index + 1) of \(model.totalSlides)"
    }
}
