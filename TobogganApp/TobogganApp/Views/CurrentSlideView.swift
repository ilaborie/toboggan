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
        }
        .padding()
        .thinCardBackground()
    }
}

#Preview {
    CurrentSlideView()
        .environmentObject(PresentationViewModel())
        .padding()
}