//
//  ConnectionSheet.swift
//  TobogganApp
//

import SwiftUI

/// Where the server lives, and what proves we may drive it.
///
/// Scanning is the primary path because the alternative is typing a LAN address
/// and a secret on a phone keyboard, while the server's homepage already shows
/// exactly the link that is needed — token and all.
struct ConnectionSheet: View {
    @Environment(ConnectionSettings.self)
    private var settings
    @Environment(PresentationModel.self)
    private var model
    @Environment(\.dismiss)
    private var dismiss

    /// Same reason as `ContentView`: two `.sheet` modifiers on one view do not
    /// both present.
    private enum ActiveSheet: String, Identifiable {
        case scanner
        case log

        var id: String { rawValue }
    }

    @State private var activeSheet: ActiveSheet?
    @State private var scanError: String?

    var body: some View {
        @Bindable var settings = settings
        NavigationStack {
            Form {
                Section {
                    Button {
                        scanError = nil
                        activeSheet = .scanner
                    } label: {
                        Label("Scan the code on the server page", systemImage: "qrcode.viewfinder")
                    }
                    if let scanError {
                        Text(scanError)
                            .font(.footnote)
                            .foregroundStyle(.red)
                    }
                } header: {
                    Text("Connect")
                } footer: {
                    Text("Open the presentation's home page on the presenting machine and scan the code it shows.")
                }

                Section("Server") {
                    TextField("http://192.168.1.10:8080", text: $settings.serverURL)
                        .textContentType(.URL)
                        .keyboardType(.URL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()

                    SecureField("Presenter token (optional)", text: $settings.presenterToken)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                }

                if settings.isLoopback {
                    Section {
                        Label(
                            """
                            This address points at the phone itself. It works in the simulator, \
                            which shares the Mac's network, but a device needs the presenting \
                            machine's address on the local network.
                            """,
                            systemImage: "exclamationmark.triangle"
                        )
                        .font(.footnote)
                    }
                }

                Section {
                    LabeledContent("Status", value: statusText)
                    if let role = model.role {
                        LabeledContent("Role", value: role == .audience ? "Watching" : "Presenting")
                    }
                    Button("Show log") { activeSheet = .log }
                }

                Section {
                    Button("Connect") {
                        model.connect(to: settings.clientURL)
                        dismiss()
                    }
                    .disabled(settings.serverURL.isEmpty)
                }
            }
            .navigationTitle("Connection")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .sheet(item: $activeSheet) { sheet in
                switch sheet {
                case .scanner: scannerSheet
                case .log: LogSheet()
                }
            }
        }
    }

    private var scannerSheet: some View {
        NavigationStack {
            QRScannerView(
                onScan: { scanned in
                    activeSheet = nil
                    if !settings.apply(scanned: scanned) {
                        scanError = "That code is not a Toboggan server address."
                        AppLog.shared.log(.ui, .error, "Unusable QR payload")
                    }
                },
                onFailure: { message in
                    activeSheet = nil
                    scanError = message
                }
            )
            .ignoresSafeArea()
            .navigationTitle("Scan")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { activeSheet = nil }
                }
            }
        }
    }

    private var statusText: String {
        switch model.connection {
        case .connecting: "Connecting…"
        case .connected: "Connected"
        case .closed: "Disconnected"
        case let .reconnecting(attempt, maxAttempt, delaySecs):
            "Reconnecting \(attempt)/\(maxAttempt) in \(delaySecs)s"
        case let .error(message): "Error: \(message)"
        }
    }
}
