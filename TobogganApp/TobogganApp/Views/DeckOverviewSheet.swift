//
//  DeckOverviewSheet.swift
//  TobogganApp
//

import SwiftUI

/// The whole deck, tap to jump.
///
/// This is what `Command.goTo` was added for: without it the only way to reach a
/// slide was to step there, which is visible on the projector.
///
/// No glass anywhere in here. A grid of cells is content, and sheets get no
/// system glass — `Glass` is not a `ShapeStyle`, so there is no such thing as a
/// glass presentation background.
struct DeckOverviewSheet: View {
    @Environment(PresentationModel.self)
    private var model
    @Environment(\.dismiss)
    private var dismiss

    private let columns = [GridItem(.adaptive(minimum: 150), spacing: 12)]

    var body: some View {
        NavigationStack {
            ScrollView {
                ScrollViewReader { proxy in
                    LazyVGrid(columns: columns, spacing: 12) {
                        // No explicit `.id()` here: the `ForEach` identity is
                        // already the index, and layering a second one on top is
                        // what `scrollTo` fails to resolve against.
                        ForEach(Array(model.deck.slides.enumerated()), id: \.offset) { index, slide in
                            cell(index: index, slide: slide)
                        }
                    }
                    .padding()
                    .task {
                        guard let current = model.currentSlideIndex else { return }
                        // A frame's grace: the grid is lazy, and on the first
                        // pass the target row does not exist for `scrollTo` to
                        // find.
                        try? await Task.sleep(for: .milliseconds(100))
                        proxy.scrollTo(current, anchor: .center)
                    }
                }
            }
            // The grid otherwise runs flush into the bottom of the screen, which
            // reads as clipped rather than as scrollable.
            .contentMargins(.bottom, 24, for: .scrollContent)
            .navigationTitle("Slides")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }

    @ViewBuilder
    private func cell(index: Int, slide: Slide) -> some View {
        let isCurrent = index == model.currentSlideIndex
        Button {
            model.goTo(slide: index)
            dismiss()
        } label: {
            VStack(alignment: .leading, spacing: 6) {
                Text("\(index + 1)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
                Text(slide.title)
                    .font(.subheadline)
                    .lineLimit(4)
                    .minimumScaleFactor(0.8)
                    .multilineTextAlignment(.leading)
                Spacer(minLength: 0)
            }
            .frame(maxWidth: .infinity, minHeight: 92, alignment: .topLeading)
            .padding(12)
            .background(.background.secondary, in: .rect(cornerRadius: 12))
            .overlay {
                if isCurrent {
                    RoundedRectangle(cornerRadius: 12).strokeBorder(.tint, lineWidth: 2)
                }
            }
        }
        .buttonStyle(.plain)
        .disabled(!model.isPresenter)
        .accessibilityLabel("Slide \(index + 1): \(slide.title)")
    }
}
