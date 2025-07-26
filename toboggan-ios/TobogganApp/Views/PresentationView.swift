import SwiftUI

struct PresentationView: View {
    @EnvironmentObject var viewModel: PresentationViewModel
    
    var body: some View {
        VStack(spacing: 0) {
            // Status bar
            StatusBarView()
                .environmentObject(viewModel)
            
            // Main slide content
            ScrollView {
                if let slide = viewModel.currentSlide {
                    SlideView(slide: slide)
                        .padding()
                } else {
                    ContentUnavailableView(
                        "No Slide",
                        systemImage: "doc.text",
                        description: Text("No slide is currently selected")
                    )
                    .padding()
                }
            }
            
            Spacer()
            
            // Controls
            ControlsView()
                .environmentObject(viewModel)
        }
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button("Disconnect") {
                    Task {
                        await viewModel.disconnect()
                    }
                }
            }
        }
    }
}

#Preview {
    NavigationView {
        PresentationView()
            .environmentObject(PresentationViewModel.preview)
    }
}