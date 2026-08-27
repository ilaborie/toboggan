//
//  RemoteBar.swift
//  TobogganApp
//

import SwiftUI

/// The floating controls, and the only glass in the app.
///
/// One `GlassEffectContainer` holding sibling capsules, with no backing plate:
/// the container's job is to merge neighbouring glass shapes into one lens, which
/// only works if the shapes are the controls themselves. A plate *and* glass
/// buttons would be the same glass-on-glass mistake as before, one level
/// shallower.
///
/// `safeAreaBar` supplies no material of its own — it is `safeAreaInset` plus
/// scroll-edge-effect propagation — so this really is the only glass here.
struct RemoteBar: View {
    @Environment(PresentationModel.self)
    private var model
    @Environment(\.accessibilityReduceTransparency)
    private var reduceTransparency
    @Namespace private var glassNamespace

    /// Translucency is a setting people turn on deliberately. `Glass.identity`
    /// leaves content as if no glass had been applied.
    private var glass: Glass {
        reduceTransparency ? .identity : .regular
    }

    var body: some View {
        VStack(spacing: 12) {
            if model.stepCount > 1 {
                StepProgress(currentStep: model.currentStep, stepCount: model.stepCount)
            }

            GlassEffectContainer(spacing: 16) {
                HStack(spacing: 12) {
                    // One button, not two branches. Only the label and the
                    // action change, so the capsule resizes as itself — no
                    // transition machinery, and no `glassEffectID`, which is for
                    // shapes that appear and disappear.
                    Button {
                        model.goPrev()
                    } label: {
                        Label(
                            model.prevIntent == .step ? "Prev Step" : "Prev Slide",
                            systemImage: "chevron.left"
                        )
                    }
                    .buttonStyle(.glass)
                    .controlSize(.large)
                    .disabled(!model.canGoPrev)
                    .animation(.smooth(duration: 0.3), value: model.prevIntent)
                    .accessibilityHint("Go to the previous \(model.prevIntent == .step ? "step" : "slide")")

                    // This one genuinely inserts and removes, which is what
                    // `glassEffectID` is for: it grows out of its neighbour
                    // instead of popping.
                    if model.isConnected, model.isPresenter {
                        Button {
                            model.send(.blink)
                        } label: {
                            Image(systemName: "bolt.fill")
                                .font(.body.weight(.semibold))
                        }
                        .buttonStyle(.plain)
                        .padding(.horizontal, 18)
                        .padding(.vertical, 14)
                        .glassEffect(glass.interactive(), in: .capsule)
                        .glassEffectID("blink", in: glassNamespace)
                        .glassEffectTransition(.matchedGeometry)
                        .accessibilityLabel("Blink the screen")
                    }

                    Button {
                        model.goNext()
                    } label: {
                        Label(
                            model.nextIntent == .step ? "Next Step" : "Next Slide",
                            systemImage: "chevron.right"
                        )
                    }
                    .buttonStyle(.glassProminent)
                    .controlSize(.large)
                    .disabled(!model.canGoNext)
                    .animation(.smooth(duration: 0.3), value: model.nextIntent)
                    .accessibilityHint("Go to the next \(model.nextIntent == .step ? "step" : "slide")")
                }
            }
        }
        .padding(.horizontal)
        .padding(.bottom, 4)
        // A remote you are not looking at should confirm by feel.
        .sensoryFeedback(.selection, trigger: model.currentSlideIndex)
        .sensoryFeedback(.impact(weight: .light), trigger: model.currentStep)
    }
}
