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
            // Play/Pause and Blink controls
            HStack(spacing: 16) {
                Button {
                    viewModel.togglePlayPause()
                } label: {
                    Label(viewModel.isPlaying ? "Pause" : "Resume", 
                          systemImage: viewModel.isPlaying ? "pause.fill" : "play.fill")
                }
                .tobogganButton(style: .primary)
                .accessibilityHint("Start or pause the presentation")
                
                Button {
                    viewModel.blink()
                } label: {
                    Label("Blink", systemImage: "bolt.fill")
                }
                .tobogganButton(style: .secondary)
                .accessibilityHint("Send blink effect")
            }
            
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
            
            // Navigation controls
            HStack(spacing: 16) {
                Button {
                    viewModel.previousSlide()
                } label: {
                    Label("Previous", systemImage: "chevron.left")
                }
                .tobogganButton(style: .secondary)
                .disabled(!viewModel.canGoPrevious)
                .accessibilityHint("Go to previous slide")
                
                Spacer()
                
                Button {
                    viewModel.nextSlide()
                } label: {
                    Label("Next", systemImage: "chevron.right")
                        .labelStyle(.titleAndIcon)
                }
                .tobogganButton(style: .primary)
                .disabled(!viewModel.canGoNext)
                .accessibilityHint("Go to next slide")
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
