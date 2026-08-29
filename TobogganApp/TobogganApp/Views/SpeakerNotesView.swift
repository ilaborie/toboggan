//
//  SpeakerNotesView.swift
//  TobogganApp
//

import SwiftUI

/// Speaker notes — the reason to look at a phone during a talk.
///
/// These reach the app for the first time: the FFI's `Slide` used to carry only
/// a title, a kind and a step count, so the phone could be a clicker and nothing
/// more. They arrive as plain text because the phone has no HTML renderer.
struct SpeakerNotesView: View {
    let notes: String

    /// The notes arrive as the plain-text projection of the slide's markup,
    /// which for a markdown-authored deck still carries its `**bold**`. Parsed
    /// inline-only so emphasis renders while the author's line breaks survive —
    /// full markdown parsing would reflow the paragraphs into one.
    private var styled: AttributedString {
        let options = AttributedString.MarkdownParsingOptions(
            interpretedSyntax: .inlineOnlyPreservingWhitespace
        )
        return (try? AttributedString(markdown: notes, options: options))
            ?? AttributedString(notes)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("Notes", systemImage: "text.alignleft")
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)

            Text(styled)
                .font(.body)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(16)
        .background(.background.secondary, in: .rect(cornerRadius: 16))
    }
}
