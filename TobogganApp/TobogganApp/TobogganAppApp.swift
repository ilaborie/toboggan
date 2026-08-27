//
//  TobogganAppApp.swift
//  TobogganApp
//

import SwiftUI

@main
struct TobogganAppApp: App {
    @State private var model = PresentationModel()
    @State private var settings = ConnectionSettings()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(model)
                .environment(settings)
        }
    }
}
