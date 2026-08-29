//
//  TobogganAppApp.swift
//  TobogganApp
//

import SwiftUI

@main
struct TobogganAppApp: App {
    @State private var model = PresentationModel()
    @State private var settings = ConnectionSettings()

    init() {
        // Before the first connection, so its diagnostics land in the same log
        // as the app's own rather than in no log at all.
        initLogging(sink: RustLogSink(), verbose: false)
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(model)
                .environment(settings)
        }
    }
}
