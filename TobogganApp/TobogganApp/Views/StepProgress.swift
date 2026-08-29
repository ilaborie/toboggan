//
//  StepProgress.swift
//  TobogganApp
//

import SwiftUI

/// Per-step dots for the current slide.
///
/// No glass and no `.interactive()`: this is a read-only indicator, and
/// interactive glass promises a tap target that does not exist.
struct StepProgress: View {
    let currentStep: Int

    /// How many dots to draw: one per state the slide can be in, which is one
    /// more than the number of reveals it has.
    let stepStates: Int

    @ScaledMetric(relativeTo: .caption)
    private var dotSize = 10

    var body: some View {
        HStack(spacing: 8) {
            ForEach(0..<stepStates, id: \.self) { step in
                Circle()
                    .fill(fill(for: step))
                    .frame(width: dotSize, height: dotSize)
                    .overlay {
                        if step > currentStep {
                            Circle().strokeBorder(.secondary.opacity(0.5), lineWidth: 1.5)
                        }
                    }
            }
        }
        .animation(.smooth(duration: 0.25), value: currentStep)
        .accessibilityElement()
        .accessibilityLabel("Step \(currentStep + 1) of \(stepStates)")
    }

    private func fill(for step: Int) -> AnyShapeStyle {
        if step < currentStep {
            return AnyShapeStyle(.tint.opacity(0.45))
        }
        if step == currentStep {
            return AnyShapeStyle(.tint)
        }
        return AnyShapeStyle(.clear)
    }
}
