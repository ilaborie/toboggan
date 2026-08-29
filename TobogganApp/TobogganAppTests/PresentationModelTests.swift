//
//  PresentationModelTests.swift
//  TobogganAppTests
//

import Foundation
import Testing
@testable import TobogganApp

@MainActor
struct PresentationModelTests {
    private func slide(_ title: String, steps: UInt32?) -> Slide {
        Slide(title: title, kind: .standard, stepCount: steps, notes: "", durationSecs: nil)
    }

    private func presenting(deck: [Slide]) -> PresentationModel {
        let model = PresentationModel()
        model.setDeckForTesting(Deck(title: "Deck", date: "2026-01-01", slides: deck))
        model.handle(connection: .connected)
        model.handle(registered: "abc", role: .presenter)
        return model
    }

    /// Before the server has said anything the client is audience, not
    /// presenter. Read off an optional as `role != .audience`, an unset role was
    /// `true` — so the app offered controls it did not have.
    @Test
    func aClientIsAudienceUntilTheServerSaysOtherwise() {
        let model = PresentationModel()
        #expect(model.isRegistered == false)
        #expect(model.isPresenter == false)
        #expect(model.canGoNext == false)
        #expect(model.canGoPrev == false)
    }

    /// An audience client offers no controls at all, rather than letting the
    /// user find out by pressing one and being refused.
    @Test
    func anAudienceClientCannotNavigate() {
        let model = PresentationModel()
        model.handle(registered: "abc", role: .audience)
        #expect(model.isPresenter == false)
        #expect(model.canGoNext == false)
        #expect(model.canGoPrev == false)
        #expect(model.notice != nil)
    }

    /// Being granted the presenter role has to clear the notice that said
    /// otherwise. Set and never cleared, it stayed pinned over working buttons.
    @Test
    func beingPromotedClearsTheWatchingNotice() {
        let model = PresentationModel()
        model.handle(registered: "abc", role: .audience)
        #expect(model.notice != nil)
        model.handle(registered: "abc", role: .presenter)
        #expect(model.notice == nil)
    }

    /// A presenter that cannot reach the server has nothing to drive, and a tap
    /// that goes nowhere is exactly what the role plumbing exists to prevent.
    @Test
    func aDisconnectedPresenterOffersNoControls() {
        let model = presenting(deck: [slide("one", steps: 0), slide("two", steps: 0)])
        model.handle(state: .running(previous: nil, current: 0, next: 1, currentStep: 0, stepCount: 0))
        #expect(model.canGoNext)

        model.handle(connection: .closed)
        #expect(model.canGoNext == false)
        #expect(model.canGoPrev == false)
    }

    /// `currentStep` runs `0...stepCount`, so a slide with three reveals has a
    /// step 3. Compared against `stepCount - 1`, the phone sent "next slide" one
    /// reveal early and skipped the last build of every slide in the deck.
    @Test
    func theLastRevealIsNotSkipped() {
        let model = presenting(deck: [slide("one", steps: 3), slide("two", steps: 0)])

        model.handle(state: .running(previous: nil, current: 0, next: 1, currentStep: 2, stepCount: 3))
        #expect(model.nextIntent == .step, "step 2 of 3 still has a reveal left")

        model.handle(state: .running(previous: nil, current: 0, next: 1, currentStep: 3, stepCount: 3))
        #expect(model.nextIntent == .slide, "step 3 of 3 is the last one")
    }

    /// A slide with no reveals moves straight on.
    @Test
    func aSlideWithoutRevealsAdvancesTheSlide() {
        let model = presenting(deck: [slide("one", steps: 0), slide("two", steps: 0)])
        model.handle(state: .running(previous: nil, current: 0, next: 1, currentStep: 0, stepCount: 0))
        #expect(model.nextIntent == .slide)
    }

    /// An uncounted slide is not a slide without reveals. Treated as one, the
    /// phone stepped over every build in the deck; asking the server for a step
    /// is right either way, because it moves to the next slide once this one
    /// runs out.
    @Test
    func anUncountedSlideAsksTheServerForAStep() {
        let model = presenting(deck: [slide("one", steps: nil), slide("two", steps: nil)])
        model.handle(state: .running(previous: nil, current: 0, next: 1, currentStep: 0, stepCount: nil))
        #expect(model.nextIntent == .step)
        #expect(model.stepStates == nil, "there are no dots to draw for a count nobody has")
    }

    /// The dots are one per state, which is one more than the reveal count.
    @Test
    func theDotsCountStatesRatherThanReveals() {
        let model = presenting(deck: [slide("one", steps: 2)])
        model.handle(state: .running(previous: nil, current: 0, next: nil, currentStep: 0, stepCount: 2))
        #expect(model.stepStates == 3)
    }

    /// Previous means the previous *step* unless we are on the first one.
    @Test
    func previousWalksBackThroughTheReveals() {
        let model = presenting(deck: [slide("one", steps: 2), slide("two", steps: 0)])
        model.handle(state: .running(previous: nil, current: 0, next: 1, currentStep: 1, stepCount: 2))
        #expect(model.prevIntent == .step)

        model.handle(state: .running(previous: nil, current: 0, next: 1, currentStep: 0, stepCount: 2))
        #expect(model.prevIntent == .slide)
    }

    /// A refusal is a permissions answer, not a broken socket — it must not
    /// raise the modal that blames the network. Classified by kind now, rather
    /// than by searching the server's English for the word "watching".
    @Test
    func aRefusalIsANoticeAndNotAnAlert() {
        let model = PresentationModel()
        model.handle(registered: "abc", role: .audience)
        model.handle(error: .server, message: "This client is watching, not presenting")
        #expect(model.notice == "This client is watching, not presenting")
        #expect(model.alert == nil)
    }

    /// The classification must not depend on the role. An audience client that
    /// cannot reach the server has a transport failure like anyone else, and it
    /// used to be demoted to a grey inline label.
    @Test
    func anAudienceClientStillSeesTransportFailuresAsFailures() {
        let model = PresentationModel()
        model.handle(registered: "abc", role: .audience)
        model.handle(error: .transport, message: "Failed to load talk: connection refused")
        #expect(model.alert == "Failed to load talk: connection refused")
    }

    @Test
    func aTransportFailureStillRaisesAnAlert() {
        let model = PresentationModel()
        model.handle(registered: "abc", role: .presenter)
        model.handle(error: .transport, message: "no route to host")
        #expect(model.alert == "no route to host")
        #expect(model.notice == nil)
    }

    /// Every log line is written `.public` and the sheet has a share button, so
    /// the connection URL must not carry the token into either.
    @Test
    func theTokenNeverReachesTheLog() {
        let redacted = PresentationModel.redactingToken("http://192.168.1.10:8080?token=s3cr3t")
        #expect(!redacted.contains("s3cr3t"))
        #expect(redacted == "http://192.168.1.10:8080")

        // Even something that is not a URL at all must not leak the query.
        #expect(!PresentationModel.redactingToken("nonsense?token=s3cr3t").contains("s3cr3t"))
        #expect(PresentationModel.redactingToken("http://192.168.1.10:8080") == "http://192.168.1.10:8080")
    }

    /// The server is authoritative: nothing moves until it says so.
    @Test
    func stateFollowsTheServerRatherThanTheTap() {
        let model = PresentationModel()
        #expect(model.currentSlideIndex == nil)
        model.handle(state: .running(
            previous: nil,
            current: 2,
            next: 3,
            currentStep: 1,
            stepCount: 4
        ))
        #expect(model.currentSlideIndex == 2)
        #expect(model.currentStep == 1)
    }

    /// Back to `init` is back to the beginning, clock included — it used to keep
    /// running from the previous talk.
    @Test
    func theInitialStateHasNoCurrentSlideAndNoClock() {
        let model = PresentationModel()
        model.handle(state: .running(previous: nil, current: 1, next: nil, currentStep: 0, stepCount: 0))
        #expect(model.startedAt != nil)

        model.handle(state: .`init`(totalSlides: 5))
        #expect(model.currentSlideIndex == nil)
        #expect(model.currentStep == 0)
        #expect(model.startedAt == nil)
    }
}
