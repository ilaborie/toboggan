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
            // Presentation title
            Text(viewModel.presentationTitle)
                .font(.headline)
                .fontWeight(.medium)
                .foregroundStyle(.primary)
                .multilineTextAlignment(.center)
                .lineLimit(2)
            
            // Connection status
            Text(viewModel.connectionStatus.displayText)
                .font(.caption)
                .foregroundStyle(viewModel.connectionStatus.color)
                .animation(.easeInOut, value: viewModel.connectionStatus)
            
            // First/Last navigation buttons
            HStack(spacing: 16) {
                Button("First Slide") {
                    viewModel.firstSlide()
                }
                .tobogganButton(style: .secondary)
                
                Spacer()
                
                Button("Last Slide") {
                    viewModel.lastSlide()
                }
                .tobogganButton(style: .secondary)
            }
        }
        .padding()
        .cardBackground()
    }
}

#Preview {
    TopBarView()
        .environmentObject(PresentationViewModel())
        .padding()
}
