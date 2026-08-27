//
//  TobogganAppTests.swift
//  TobogganAppTests
//

import Testing
@testable import TobogganApp

struct TobogganAppTests {
    @Test
    func uniffiInitialisesWithoutChecksumMismatch() async {
        let config = ClientConfig(url: "http://localhost:8080", maxRetries: 3, retryDelay: 1.0)
        let client = TobogganClient(
            config: config,
            clientName: "iOS Test",
            handler: TestNotificationHandler()
        )

        // This used to be unsatisfiable: `is_connected` returned `true`
        // unconditionally on the Rust side, so a client that had never opened a
        // socket claimed to be connected.
        #expect(client.isConnected() == false)
    }

    @Test
    func commandsCoverTheProtocol() async {
        let commands: [Command] = [
            .next, .previous, .first, .last, .nextStep, .previousStep, .blink, .goTo(slide: 3)
        ]
        #expect(commands.count == 8)
        #expect(commands.contains(.goTo(slide: 3)))
    }
}

// MARK: - ConnectionSettings

@MainActor
struct ConnectionSettingsTests {
    @Test
    func aTokenIsAppendedAsAQueryParameter() {
        let settings = ConnectionSettings()
        settings.serverURL = "http://192.168.1.10:8080"
        settings.presenterToken = "s3cr3t"
        #expect(settings.clientURL == "http://192.168.1.10:8080?token=s3cr3t")
    }

    @Test
    func noTokenLeavesTheAddressAlone() {
        let settings = ConnectionSettings()
        settings.serverURL = "http://192.168.1.10:8080"
        settings.presenterToken = ""
        #expect(settings.clientURL == "http://192.168.1.10:8080")
    }

    /// A token with a space or a `+` has to survive the round trip; the server
    /// and the web client disagreed about this once already.
    @Test
    func anAwkwardTokenIsPercentEncoded() {
        let settings = ConnectionSettings()
        settings.serverURL = "http://192.168.1.10:8080"
        settings.presenterToken = "a b+c"
        #expect(settings.clientURL == "http://192.168.1.10:8080?token=a%20b%2Bc")
    }

    /// One scan has to configure both fields: the server's link carries the
    /// token in its query.
    @Test
    func scanningALinkSetsTheAddressAndTheToken() {
        let settings = ConnectionSettings()
        #expect(settings.apply(scanned: "http://192.168.1.10:8080/run?token=s3cr3t"))
        #expect(settings.serverURL == "http://192.168.1.10:8080")
        #expect(settings.presenterToken == "s3cr3t")
    }

    @Test
    func scanningSomethingElseIsRejected() {
        let settings = ConnectionSettings()
        #expect(settings.apply(scanned: "WIFI:S:somenetwork;T:WPA;") == false)
    }

    @Test
    func loopbackIsRecognised() {
        let settings = ConnectionSettings()
        settings.serverURL = "http://127.0.0.1:8080"
        #expect(settings.isLoopback)
        settings.serverURL = "http://192.168.1.10:8080"
        #expect(settings.isLoopback == false)
    }
}

// MARK: - PresentationModel

@MainActor
struct PresentationModelTests {
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

    /// A refusal is a permissions answer, not a broken socket — it must not
    /// raise the modal that blames the network.
    @Test
    func aRefusalIsANoticeAndNotAnAlert() {
        let model = PresentationModel()
        model.handle(registered: "abc", role: .audience)
        model.handle(error: "This client is watching, not presenting")
        #expect(model.notice == "This client is watching, not presenting")
        #expect(model.alert == nil)
    }

    @Test
    func aTransportFailureStillRaisesAnAlert() {
        let model = PresentationModel()
        model.handle(registered: "abc", role: .presenter)
        model.handle(error: "no route to host")
        #expect(model.alert == "no route to host")
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

    @Test
    func theInitialStateHasNoCurrentSlide() {
        let model = PresentationModel()
        model.handle(state: .running(previous: nil, current: 1, next: nil, currentStep: 0, stepCount: 0))
        model.handle(state: .`init`(totalSlides: 5))
        #expect(model.currentSlideIndex == nil)
        #expect(model.currentStep == 0)
    }
}

final class TestNotificationHandler: ClientNotificationHandler, @unchecked Sendable {
    func onStateChange(state: PresentationState) {}
    func onTalkChange(state: PresentationState) {}
    func onConnectionStatusChange(status: ConnectionStatus) {}
    func onError(error: String) {}
    func onRegistered(clientId: String, role: ClientRole) {}
    func onClientConnected(clientId: String, name: String) {}
    func onClientDisconnected(clientId: String, name: String) {}
}
