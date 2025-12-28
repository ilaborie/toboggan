//
//  NavigationControlsView.swift
//  TobogganApp
//
//  Created by Igor Laborie on 16/08/2025.
//

import SwiftUI

struct NavigationControlsView: View {
    @EnvironmentObject var viewModel: PresentationViewModel
    
    var body: some View {
        VStack(spacing: 20) {
            // Blink control with modern glass button
            Button {
                viewModel.blink()
            } label: {
                Label("Blink Screen", systemImage: "bolt.fill")
                    .font(.body.weight(.semibold))
            }
            .tobogganButton(style: .secondary)
            .accessibilityHint("Send blink effect to presentation")
            
            // Next slide preview with enhanced glass styling
            VStack(alignment: .leading, spacing: 10) {
                Label {
                    Text("Up Next")
                        .font(.subheadline)
                        .fontWeight(.semibold)
                        .foregroundStyle(.secondary)
                } icon: {
                    Image(systemName: "arrow.right.circle.fill")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                
                Text(viewModel.nextSlideTitle)
                    .font(.body)
                    .fontWeight(.medium)
                    .foregroundStyle(.primary)
                    .multilineTextAlignment(.leading)
                    .lineLimit(3)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .leading)
            .glassEffect(.regular, in: .rect(cornerRadius: 16))
            
            // Smart step/slide navigation controls
            HStack(spacing: 16) {
                // Previous button: Step or Slide depending on position
                if viewModel.showPreviousSlideInsteadOfStep {
                    // On first step, show Previous Slide button (or nothing if no previous slide)
                    if viewModel.canGoPrevious {
                        Button {
                            withAnimation(.spring(response: 0.3, dampingFraction: 0.7)) {
                                viewModel.previousSlide()
                            }
                        } label: {
                            Label("Prev Slide", systemImage: "chevron.left")
                        }
                        .tobogganButton(style: .secondary)
                        .accessibilityHint("Go to previous slide")
                        .transition(.scale.combined(with: .opacity))
                    }
                } else {
                    // Not on first step, show Previous Step button
                    Button {
                        withAnimation(.spring(response: 0.3, dampingFraction: 0.7)) {
                            viewModel.previousStep()
                        }
                    } label: {
                        Label("Prev Step", systemImage: "chevron.left")
                    }
                    .tobogganButton(style: .secondary)
                    .disabled(!viewModel.canGoPreviousStep)
                    .accessibilityHint("Go to previous step")
                    .transition(.scale.combined(with: .opacity))
                }

                Spacer()

                // Next button: Step or Slide depending on position
                if viewModel.showNextSlideInsteadOfStep {
                    // On last step, show Next Slide button (or nothing if no next slide)
                    if viewModel.canGoNext {
                        Button {
                            withAnimation(.spring(response: 0.3, dampingFraction: 0.7)) {
                                viewModel.nextSlide()
                            }
                        } label: {
                            Label("Next Slide", systemImage: "chevron.right")
                                .labelStyle(.titleAndIcon)
                        }
                        .tobogganButton(style: .primary)
                        .accessibilityHint("Go to next slide")
                        .transition(.scale.combined(with: .opacity))
                    }
                } else {
                    // Not on last step, show Next Step button
                    Button {
                        withAnimation(.spring(response: 0.3, dampingFraction: 0.7)) {
                            viewModel.nextStep()
                        }
                    } label: {
                        Label("Next Step", systemImage: "chevron.right")
                            .labelStyle(.titleAndIcon)
                    }
                    .tobogganButton(style: .primary)
                    .disabled(!viewModel.canGoNextStep)
                    .accessibilityHint("Go to next step")
                    .transition(.scale.combined(with: .opacity))
                }
            }
        }
        .thinCardBackground()
    }
}

#Preview {
    NavigationControlsView()
        .environmentObject(PresentationViewModel())
        .padding()
}
