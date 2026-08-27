//
//  UpNextCard.swift
//  TobogganApp
//

import SwiftUI

/// What comes after this slide, so the next sentence can start before the deck
/// catches up.
struct UpNextCard: View {
    let title: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Label("Up next", systemImage: "arrow.right")
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)

            Text(title ?? "End of the presentation")
                .font(.body.weight(.medium))
                .foregroundStyle(title == nil ? .secondary : .primary)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(16)
        .background(.background.secondary, in: .rect(cornerRadius: 16))
    }
}
