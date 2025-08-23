//
//  ViewExtensions.swift
//  TobogganApp
//
//  Created by Igor Laborie on 23/08/2025.
//

import SwiftUI

// MARK: - Common View Modifiers
extension View {
    func cardBackground() -> some View {
        self.background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
    }
    
    func thinCardBackground() -> some View {
        self.background(.thinMaterial, in: RoundedRectangle(cornerRadius: 16))
    }
}

// MARK: - Common Button Modifiers
extension View {
    func tobogganButton(style: TobogganButtonType = .secondary) -> some View {
        Group {
            if style == .primary {
                self
                    .buttonStyle(BorderedProminentButtonStyle())
                    .controlSize(.large)
            } else {
                self
                    .buttonStyle(BorderedButtonStyle())
                    .controlSize(.large)
            }
        }
    }
}

enum TobogganButtonType {
    case primary
    case secondary
}