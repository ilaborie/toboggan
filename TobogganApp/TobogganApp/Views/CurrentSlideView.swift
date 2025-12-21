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
            // First line: Duration and slide progress
            HStack {
                Text(viewModel.formattedDuration)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)

                Spacer()

                Text(slideProgressText)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            // Center: Slide title
            VStack {
                Spacer()

                Text(slideTitle)
                    .font(.largeTitle)
                    .fontWeight(.bold)
                    .foregroundStyle(viewModel.currentSlide != nil ? .primary : .secondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: .infinity)

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

// Step indicator circles view
struct StepIndicatorView: View {
    let currentStep: Int
    let stepCount: Int

    private let circleSize: CGFloat = 10

    var body: some View {
        HStack(spacing: 8) {
            ForEach(0..<stepCount, id: \.self) { step in
                stepCircle(for: step)
                    .frame(width: circleSize, height: circleSize)
            }
        }
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private func stepCircle(for step: Int) -> some View {
        if step < currentStep {
            // Done: filled circle in text color
            Circle()
                .fill(Color.primary)
        } else if step == currentStep {
            // Current: filled circle in accent color
            Circle()
                .fill(Color.accentColor)
        } else {
            // Remaining: unfilled circle
            Circle()
                .stroke(Color.secondary, lineWidth: 1.5)
        }
    }
}

#Preview {
    CurrentSlideView()
        .environmentObject(PresentationViewModel())
        .padding()
}
