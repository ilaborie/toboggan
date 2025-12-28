//
//  TopBarView.swift
//  TobogganApp
//
//  Created by Igor Laborie on 16/08/2025.
//

import SwiftUI

struct TopBarView: View {
    @EnvironmentObject var viewModel: PresentationViewModel
    
    var body: some View {
        VStack(spacing: 12) {
            // Presentation title with modern styling
            Text(viewModel.presentationTitle)
                .font(.headline)
                .fontWeight(.semibold)
                .foregroundStyle(.primary)
                .multilineTextAlignment(.center)
                .lineLimit(2)
            
            // Connection status with animated color badge
            HStack(spacing: 6) {
                Circle()
                    .fill(viewModel.connectionStatus.color)
                    .frame(width: 8, height: 8)
                    .animation(.easeInOut, value: viewModel.connectionStatus)
                
                Text(viewModel.connectionStatus.displayText)
                    .font(.caption)
                    .fontWeight(.medium)
                    .foregroundStyle(.secondary)
                    .animation(.easeInOut, value: viewModel.connectionStatus)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .glassEffect(.regular, in: .capsule)
            
            // First/Last navigation buttons with Liquid Glass
            HStack(spacing: 16) {
                Button {
                    viewModel.firstSlide()
                } label: {
                    Label("First", systemImage: "arrow.backward.to.line")
                }
                .tobogganButton(style: .secondary)
                
                Spacer()
                
                Button {
                    viewModel.lastSlide()
                } label: {
                    Label("Last", systemImage: "arrow.forward.to.line")
                }
                .tobogganButton(style: .secondary)
            }
        }
        .cardBackground()
    }
}

#Preview {
    TopBarView()
        .environmentObject(PresentationViewModel())
        .padding()
}
