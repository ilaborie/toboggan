//
//  ViewExtensions.swift
//  TobogganApp
//
//  Created by Igor Laborie on 23/08/2025.
//

import SwiftUI

// MARK: - Liquid Glass View Modifiers
extension View {
    /// Applies a subtle Liquid Glass effect with rounded corners
    func cardBackground() -> some View {
        self.padding()
            .glassEffect(.regular, in: .rect(cornerRadius: 16))
    }
    
    /// Applies an interactive Liquid Glass effect with more prominent corners
    func thinCardBackground() -> some View {
        self.padding()
            .glassEffect(.regular.interactive(), in: .rect(cornerRadius: 20))
    }
    
    /// Applies a tinted Liquid Glass effect
    func tintedGlassBackground(tint: Color) -> some View {
        self.padding()
            .glassEffect(.regular.tint(tint).interactive(), in: .rect(cornerRadius: 20))
    }
}

// MARK: - Modern Button Modifiers
extension View {
    func tobogganButton(style: TobogganButtonType = .secondary) -> some View {
        Group {
            if style == .primary {
                self
                    .buttonStyle(.glassProminent) // Use Liquid Glass prominent style
                    .controlSize(.large)
            } else {
                self
                    .buttonStyle(.glass) // Use Liquid Glass style
                    .controlSize(.large)
            }
        }
    }
}

enum TobogganButtonType {
    case primary
    case secondary
}
