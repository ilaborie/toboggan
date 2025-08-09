import SwiftUI

struct StatusBarView: View {
    @EnvironmentObject var viewModel: PresentationViewModel
    
    var body: some View {
        HStack {
            // Connection status
            HStack(spacing: 6) {
                Circle()
                    .fill(connectionColor)
                    .frame(width: 8, height: 8)
                Text(connectionText)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            // Presentation info
            if let talkInfo = viewModel.talkInfo {
                VStack(alignment: .center, spacing: 2) {
                    Text(talkInfo.title)
                        .font(.caption)
                        .fontWeight(.medium)
                        .lineLimit(1)
                    
                    if let date = talkInfo.date {
                        Text(date)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
            
            Spacer()
            
            // Slide counter and state
            HStack(spacing: 8) {
                if let talkInfo = viewModel.talkInfo,
                   let currentSlide = viewModel.currentSlide {
                    let currentIndex = slideIndex(for: currentSlide.id, in: talkInfo.slides)
                    Text("\\(currentIndex + 1)/\\(talkInfo.slides.count)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                
                // State indicator
                stateIndicator
            }
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
        .background(Color(.systemGray6))
    }
    
    private var connectionColor: Color {
        switch viewModel.connectionStatus {
        case .connected:
            return .green
        case .connecting:
            return .orange
        case .disconnected:
            return .gray
        case .error:
            return .red
        }
    }
    
    private var connectionText: String {
        switch viewModel.connectionStatus {
        case .connected:
            return "Connected"
        case .connecting:
            return "Connecting..."
        case .disconnected:
            return "Disconnected"
        case .error:
            return "Error"
        }
    }
    
    @ViewBuilder
    private var stateIndicator: some View {
        if let state = viewModel.presentationState {
            HStack(spacing: 4) {
                Image(systemName: stateIcon(for: state))
                    .font(.caption)
                Text(stateText(for: state))
                    .font(.caption)
            }
            .foregroundColor(stateColor(for: state))
        }
    }
    
    private func stateIcon(for state: State) -> String {
        switch state {
        case .running:
            return "play.fill"
        case .paused:
            return "pause.fill"
        case .done:
            return "checkmark.circle.fill"
        }
    }
    
    private func stateText(for state: State) -> String {
        switch state {
        case .running:
            return "Playing"
        case .paused:
            return "Paused"
        case .done:
            return "Done"
        }
    }
    
    private func stateColor(for state: State) -> Color {
        switch state {
        case .running:
            return .green
        case .paused:
            return .orange
        case .done:
            return .blue
        }
    }
    
    private func slideIndex(for slideId: String, in slides: [Slide]) -> Int {
        slides.firstIndex { $0.id == slideId } ?? 0
    }
}

#Preview {
    VStack {
        StatusBarView()
            .environmentObject(PresentationViewModel.preview)
        Spacer()
    }
}