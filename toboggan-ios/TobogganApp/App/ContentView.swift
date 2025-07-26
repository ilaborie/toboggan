import SwiftUI

struct ContentView: View {
    @StateObject private var viewModel = PresentationViewModel()
    
    var body: some View {
        NavigationView {
            Group {
                if viewModel.isConnected {
                    PresentationView()
                        .environmentObject(viewModel)
                } else {
                    ConnectionView()
                        .environmentObject(viewModel)
                }
            }
            .navigationTitle("Toboggan")
            .navigationBarTitleDisplayMode(.inline)
        }
        .alert("Error", isPresented: .constant(viewModel.errorMessage != nil)) {
            Button("OK") {
                viewModel.clearError()
            }
        } message: {
            Text(viewModel.errorMessage ?? "")
        }
    }
}

struct ConnectionView: View {
    @EnvironmentObject var viewModel: PresentationViewModel
    @State private var serverUrl = "ws://localhost:3000/api/ws"
    
    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "tv")
                .font(.system(size: 80))
                .foregroundColor(.blue)
            
            Text("Welcome to Toboggan")
                .font(.largeTitle)
                .fontWeight(.bold)
            
            Text("Connect to your presentation server")
                .font(.body)
                .foregroundColor(.secondary)
            
            VStack(alignment: .leading, spacing: 8) {
                Text("Server URL")
                    .font(.headline)
                
                TextField("ws://localhost:3000/api/ws", text: $serverUrl)
                    .textFieldStyle(RoundedBorderTextFieldStyle())
                    .autocapitalization(.none)
                    .disableAutocorrection(true)
            }
            
            Button(action: {
                Task {
                    await viewModel.connect(to: serverUrl)
                }
            }) {
                HStack {
                    if viewModel.isConnecting {
                        ProgressView()
                            .scaleEffect(0.8)
                        Text("Connecting...")
                    } else {
                        Image(systemName: "wifi")
                        Text("Connect")
                    }
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(Color.blue)
                .foregroundColor(.white)
                .cornerRadius(10)
            }
            .disabled(viewModel.isConnecting || serverUrl.isEmpty)
            
            Spacer()
        }
        .padding()
    }
}

#Preview {
    ContentView()
}