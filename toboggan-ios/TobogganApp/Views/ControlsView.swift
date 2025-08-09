import SwiftUI

struct ControlsView: View {
    @EnvironmentObject var viewModel: PresentationViewModel
    
    var body: some View {
        VStack(spacing: 16) {
            // Navigation controls
            HStack(spacing: 20) {
                // First slide
                Button(action: viewModel.firstSlide) {
                    Image(systemName: "backward.end.fill")
                        .font(.title2)
                }
                .buttonStyle(ControlButtonStyle())
                
                // Previous slide
                Button(action: viewModel.previousSlide) {
                    Image(systemName: "backward.fill")
                        .font(.title2)
                }
                .buttonStyle(ControlButtonStyle())
                
                Spacer()
                
                // Play/Pause toggle
                Button(action: togglePlayPause) {
                    Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                        .font(.title)
                }
                .buttonStyle(PlayPauseButtonStyle(isPlaying: isPlaying))
                
                Spacer()
                
                // Next slide
                Button(action: viewModel.nextSlide) {
                    Image(systemName: "forward.fill")
                        .font(.title2)
                }
                .buttonStyle(ControlButtonStyle())
                
                // Last slide
                Button(action: viewModel.lastSlide) {
                    Image(systemName: "forward.end.fill")
                        .font(.title2)
                }
                .buttonStyle(ControlButtonStyle())
            }
            .padding(.horizontal)
        }
        .padding(.vertical)
        .background(Color(.systemBackground))
        .overlay(
            Rectangle()
                .frame(height: 0.5)
                .foregroundColor(Color(.separator)),
            alignment: .top
        )
    }
    
    private var isPlaying: Bool {
        if case .running = viewModel.presentationState {
            return true
        }
        return false
    }
    
    private func togglePlayPause() {
        if isPlaying {
            viewModel.pausePresentation()
        } else {
            viewModel.resumePresentation()
        }
    }
}

struct ControlButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .frame(width: 44, height: 44)
            .background(
                Circle()
                    .fill(Color(.systemGray5))
                    .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            )
            .foregroundColor(.primary)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}

struct PlayPauseButtonStyle: ButtonStyle {
    let isPlaying: Bool
    
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .frame(width: 60, height: 60)
            .background(
                Circle()
                    .fill(isPlaying ? Color.orange : Color.green)
                    .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            )
            .foregroundColor(.white)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}

#Preview {
    VStack {
        Spacer()
        ControlsView()
            .environmentObject(PresentationViewModel.preview)
    }
}