//
//  ContentView.swift
//  TobogganApp
//
//  Created by Igor Laborie on 16/08/2025.
//

import SwiftUI

// Main ContentView with modern Liquid Glass design
struct ContentView: View {
    @StateObject private var viewModel = PresentationViewModel()
    
    var body: some View {
        NavigationView {
            // Wrap all glass effects in GlassEffectContainer for better performance and morphing
            GlassEffectContainer(spacing: 20) {
                VStack(spacing: 0) {
                    // Top Section: Title and controls
                    TopBarView()
                        .environmentObject(viewModel)
                        .padding()
                    
                    // Middle Section: Current Slide Display (expands to fill space)
                    CurrentSlideView()
                        .environmentObject(viewModel)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .padding(.horizontal)
                    
                    // Bottom Section: Navigation Controls (always at bottom)
                    NavigationControlsView()
                        .environmentObject(viewModel)
                        .padding()
                }
            }
            .navigationTitle("Toboggan")
            .navigationBarTitleDisplayMode(.inline)
        }
        .alert("Connection Error", isPresented: $viewModel.showErrorAlert) {
            Button("OK", role: .cancel) { }
        } message: {
            Text(viewModel.errorMessage)
        }
    }
}

#Preview {
    ContentView()
}
