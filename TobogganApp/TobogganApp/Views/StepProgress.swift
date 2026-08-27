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
    let stepCount: Int

    @ScaledMetric(relativeTo: .caption)
    private var dotSize = 10

    var body: some View {
        HStack(spacing: 8) {
            ForEach(0..<stepCount, id: \.self) { step in
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
        .accessibilityLabel("Step \(currentStep + 1) of \(stepCount)")
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
