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
            // Blink control
            Button {
                viewModel.blink()
            } label: {
                Label("Blink", systemImage: "bolt.fill")
            }
            .tobogganButton(style: .secondary)
            .accessibilityHint("Send blink effect")
            
            // Next slide preview
            VStack(alignment: .leading, spacing: 8) {
                Text("Next Slide")
                    .font(.subheadline)
                    .fontWeight(.medium)
                    .foregroundStyle(.secondary)
                
                Text(viewModel.nextSlideTitle)
                    .font(.body)
                    .foregroundStyle(.primary)
                    .multilineTextAlignment(.leading)
                    .lineLimit(3)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding()
            .cardBackground()
            
            // Step navigation controls
            HStack(spacing: 16) {
                Button {
                    viewModel.previousStep()
                } label: {
                    Label("Prev Step", systemImage: "chevron.left")
                }
                .tobogganButton(style: .secondary)
                .disabled(!viewModel.canGoPreviousStep)
                .accessibilityHint("Go to previous step")

                Spacer()

                Button {
                    viewModel.nextStep()
                } label: {
                    Label("Next Step", systemImage: "chevron.right")
                        .labelStyle(.titleAndIcon)
                }
                .tobogganButton(style: .primary)
                .disabled(!viewModel.canGoNextStep)
                .accessibilityHint("Go to next step")
            }
        }
        .padding()
        .thinCardBackground()
    }
}

#Preview {
    NavigationControlsView()
        .environmentObject(PresentationViewModel())
        .padding()
}
