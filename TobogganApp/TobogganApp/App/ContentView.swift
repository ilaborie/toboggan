//
//  ContentView.swift
//  TobogganApp
//

import SwiftUI

/// The deck as content, with the controls floating over it.
///
/// The previous layout put three glass cards in a fixed `VStack` over a
/// hand-rolled gradient. Glass refracts what is behind it, and a flat two-stop
/// gradient refracts to a flat two-stop gradient — so nothing read as glass while
/// the real content sat inside it. Here the notes and the title scroll *under*
/// the bar and the toolbar, which is what gives the material something to do.
struct ContentView: View {
    @Environment(PresentationModel.self)
    private var model
    @Environment(ConnectionSettings.self)
    private var settings

    /// One sheet binding, not two.
    ///
    /// Two `.sheet` modifiers on the same view do not both work — the later one
    /// wins and the earlier never presents, which is why the slide overview
    /// could not be opened at all.
    private enum ActiveSheet: String, Identifiable {
        case overview
        case connection

        var id: String { rawValue }
    }

    @State private var activeSheet: ActiveSheet?

    var body: some View {
        @Bindable var model = model
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    if let notice = model.notice {
                        // Inline, not modal: a refusal is a permissions answer,
                        // not a broken connection.
                        Label(notice, systemImage: "eye")
                            .font(.footnote)
                            .padding(12)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(.background.secondary, in: .rect(cornerRadius: 12))
                    }

                    SlideHero()

                    if let notes = model.currentSlide?.notes, !notes.isEmpty {
                        SpeakerNotesView(notes: notes)
                    }

                    UpNextCard(title: model.nextSlide?.title)
                }
                .padding()
            }
            // Do not also set `.bottom`: `safeAreaBar` already extends that edge.
            .scrollEdgeEffectStyle(.soft, for: .top)
            .safeAreaBar(edge: .bottom) {
                RemoteBar()
            }
            .navigationTitle(model.deck.title.isEmpty ? "Toboggan" : model.deck.title)
            .navigationBarTitleDisplayMode(.inline)
            // No `.toolbarBackground(.hidden)` and no forced dark scheme: the
            // system's shared toolbar glass is what we want, and the forced dark
            // chrome was what made `.primary` unreadable in light mode.
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button {
                        activeSheet = .connection
                    } label: {
                        Label("Connection", systemImage: connectionSymbol)
                    }
                    .tint(connectionTint)
                }
                ToolbarSpacer(.flexible, placement: .topBarTrailing)
                ToolbarItem(placement: .topBarTrailing) {
                    Button {
                        activeSheet = .overview
                    } label: {
                        Label("Slides", systemImage: "square.grid.2x2")
                    }
                    .disabled(model.totalSlides == 0)
                }
            }
        }
        .sheet(item: $activeSheet) { sheet in
            switch sheet {
            case .overview: DeckOverviewSheet()
            case .connection: ConnectionSheet()
            }
        }
        .alert(
            "Connection Error",
            isPresented: Binding(
                get: { model.alert != nil },
                set: { if !$0 { model.alert = nil } }
            )
        ) {
            Button("OK", role: .cancel) {}
            Button("Settings") { activeSheet = .connection }
        } message: {
            Text(model.alert ?? "")
        }
        .task {
            model.connect(to: settings.clientURL)
        }
    }

    private var connectionSymbol: String {
        switch model.connection {
        case .connected: "wifi"
        case .connecting, .reconnecting: "wifi.exclamationmark"
        case .closed, .error: "wifi.slash"
        }
    }

    private var connectionTint: Color {
        switch model.connection {
        case .connected: .green
        case .connecting, .reconnecting: .orange
        case .closed, .error: .red
        }
    }
}
