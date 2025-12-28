//
//  CurrentSlideView.swift
//  TobogganApp
//
//  Created by Igor Laborie on 16/08/2025.
//

import SwiftUI

struct CurrentSlideView: View {
    @EnvironmentObject var viewModel: PresentationViewModel

    private var slideProgressText: String {
        if let currentIndex = viewModel.currentSlideIndex {
            return "\(currentIndex + 1) of \(viewModel.totalSlides)"
        } else {
            return "Ready (\(viewModel.totalSlides) slides)"
        }
    }

    private var slideTitle: String {
        if let slide = viewModel.currentSlide {
            return slide.title
        } else {
            return "Ready to Start"
        }
    }

    var body: some View {
        VStack(spacing: 16) {
            // First line: Duration and slide progress with glass badges
            HStack(spacing: 12) {
                // Duration badge
                Label {
                    Text(viewModel.formattedDuration)
                        .font(.subheadline)
                        .fontWeight(.medium)
                } icon: {
                    Image(systemName: "clock.fill")
                        .font(.caption)
                }
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .glassEffect(.regular, in: .capsule)

                Spacer()

                // Slide progress badge
                Label {
                    Text(slideProgressText)
                        .font(.subheadline)
                        .fontWeight(.medium)
                } icon: {
                    Image(systemName: "square.stack.3d.up.fill")
                        .font(.caption)
                }
                .foregroundStyle(.secondary)
                .padding(.horizontal, 12)
                .padding(.vertical, 6)
                .glassEffect(.regular, in: .capsule)
            }

            // Center: Slide title with modern typography
            VStack {
                Spacer()

                Text(slideTitle)
                    .font(.system(size: 34, weight: .bold, design: .rounded))
                    .foregroundStyle(viewModel.currentSlide != nil ? .primary : .secondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: .infinity)
                    .padding(.horizontal)

                Spacer()
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            // Step indicators (only show if there are steps)
            if viewModel.stepCount > 0 {
                StepIndicatorView(
                    currentStep: viewModel.currentStep,
                    stepCount: viewModel.stepCount
                )
            }
        }
        .padding()
        .thinCardBackground()
    }
}

// Modern step indicator with Liquid Glass effect
struct StepIndicatorView: View {
    let currentStep: Int
    let stepCount: Int

    private let circleSize: CGFloat = 12

    var body: some View {
        HStack(spacing: 10) {
            ForEach(0..<stepCount, id: \.self) { step in
                stepCircle(for: step)
                    .frame(width: circleSize, height: circleSize)
                    .animation(.spring(response: 0.3, dampingFraction: 0.7), value: currentStep)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .glassEffect(.regular.interactive(), in: .capsule)
    }

    @ViewBuilder
    private func stepCircle(for step: Int) -> some View {
        if step < currentStep {
            // Done: filled circle with checkmark
            ZStack {
                Circle()
                    .fill(Color.green.gradient)
                Image(systemName: "checkmark")
                    .font(.system(size: 6, weight: .bold))
                    .foregroundStyle(.white)
            }
        } else if step == currentStep {
            // Current: pulsing filled circle with accent color
            Circle()
                .fill(Color.accentColor.gradient)
                .shadow(color: .accentColor.opacity(0.5), radius: 4)
        } else {
            // Remaining: subtle circle
            Circle()
                .stroke(Color.secondary.opacity(0.5), lineWidth: 2)
        }
    }
}

#Preview {
    CurrentSlideView()
        .environmentObject(PresentationViewModel())
        .padding()
}
