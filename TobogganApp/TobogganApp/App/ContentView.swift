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
            ZStack {
                // Gradient background for glass effect depth
                LinearGradient(
                    colors: [
                        Color(red: 0.0, green: 0.0, blue: 0.5),      // Navy (#000080)
                        Color(red: 0.56, green: 0.27, blue: 0.52)    // Plum (#8E4585)
                    ],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
                .ignoresSafeArea()

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
            }
            .navigationTitle("Toboggan")
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(.hidden, for: .navigationBar)
            .toolbarColorScheme(.dark, for: .navigationBar)
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
