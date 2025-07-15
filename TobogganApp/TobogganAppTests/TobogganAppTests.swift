//
//  TobogganAppTests.swift
//  TobogganAppTests
//
//  Created by Igor Laborie on 16/08/2025.
//

import Testing
@testable import TobogganApp

struct TobogganAppTests {
    @Test
    func testUniffiInitialization() async {
        // Test that UniFFI initialization works without checksum mismatch
        let config = ClientConfig(
            url: "http://localhost:8080",
            maxRetries: 3,
            retryDelay: 1000
        )
        
        let testHandler = TestNotificationHandler()
        // This should not crash with UniFFI checksum mismatch
        let client = TobogganClient(config: config, handler: testHandler)
        
        // Verify client was created successfully
        #expect(client.isConnected() == false) // Should be false initially
    }

    @Test
    func testCommandEnum() async {
        // Test that Command enum values work correctly
        let commands: [Command] = [.next, .previous, .first, .last, .pause, .resume, .blink]
        
        // Should not crash when accessing enum values
        #expect(commands.count == 8)
    }
}

final class TestNotificationHandler: ClientNotificationHandler, @unchecked Sendable {
    init() {
        print("🔔 iOS: NotificationHandler initialized")
    }
    
    func onStateChange(state: State) {
        print("🔔 iOS: NotificationHandler.onStateChange called with state: \(state)")
    }
    
    func onConnectionStatusChange(status: ConnectionStatus) {
        print("🔔 iOS: NotificationHandler.onConnectionStatusChange called with status: \(status)")
    }
    
    func onError(error: String) {
        print("🔔 iOS: NotificationHandler.onError called with error: \(error)")
    }
}
