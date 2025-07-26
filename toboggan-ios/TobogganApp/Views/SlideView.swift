import SwiftUI

struct SlideView: View {
    let slide: Slide
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            // Slide kind indicator
            HStack {
                slideKindBadge
                Spacer()
                Text("ID: \\(slide.id)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            // Title
            Text(slide.title)
                .font(titleFont)
                .fontWeight(.bold)
                .multilineTextAlignment(textAlignment)
            
            // Body content
            SlideContentView(content: slide.body, style: slide.style)
            
            // Notes (if available)
            if let notes = slide.notes, !notes.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Divider()
                    
                    HStack {
                        Image(systemName: "note.text")
                            .foregroundColor(.orange)
                        Text("Speaker Notes")
                            .font(.headline)
                            .foregroundColor(.orange)
                    }
                    
                    Text(notes)
                        .font(.body)
                        .foregroundColor(.secondary)
                        .padding()
                        .background(Color.orange.opacity(0.1))
                        .cornerRadius(8)
                }
            }
        }
        .padding()
        .background(backgroundColor)
        .cornerRadius(12)
        .shadow(radius: 2)
    }
    
    private var slideKindBadge: some View {
        HStack(spacing: 4) {
            Image(systemName: slideKindIcon)
            Text(slideKindText)
        }
        .font(.caption)
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(slideKindColor.opacity(0.2))
        .foregroundColor(slideKindColor)
        .cornerRadius(8)
    }
    
    private var slideKindIcon: String {
        switch slide.kind {
        case .cover:
            return "star.fill"
        case .part:
            return "folder.fill"
        case .standard:
            return "doc.text.fill"
        }
    }
    
    private var slideKindText: String {
        switch slide.kind {
        case .cover:
            return "Cover"
        case .part:
            return "Part"
        case .standard:
            return "Standard"
        }
    }
    
    private var slideKindColor: Color {
        switch slide.kind {
        case .cover:
            return .purple
        case .part:
            return .blue
        case .standard:
            return .green
        }
    }
    
    private var titleFont: Font {
        switch slide.kind {
        case .cover:
            return .largeTitle
        case .part:
            return .title
        case .standard:
            return .title2
        }
    }
    
    private var textAlignment: TextAlignment {
        switch slide.style {
        case .center:
            return .center
        case .default, .code:
            return .leading
        }
    }
    
    private var backgroundColor: Color {
        switch slide.kind {
        case .cover:
            return Color.purple.opacity(0.05)
        case .part:
            return Color.blue.opacity(0.05)
        case .standard:
            return Color.green.opacity(0.05)
        }
    }
}

struct SlideContentView: View {
    let content: String
    let style: SlideStyle
    
    var body: some View {
        VStack(alignment: alignment, spacing: 12) {
            Text(content)
                .font(contentFont)
                .multilineTextAlignment(textAlignment)
                .lineLimit(nil)
        }
        .frame(maxWidth: .infinity, alignment: frameAlignment)
    }
    
    private var contentFont: Font {
        switch style {
        case .code:
            return .system(.body, design: .monospaced)
        case .default, .center:
            return .body
        }
    }
    
    private var textAlignment: TextAlignment {
        switch style {
        case .center:
            return .center
        case .default, .code:
            return .leading
        }
    }
    
    private var alignment: HorizontalAlignment {
        switch style {
        case .center:
            return .center
        case .default, .code:
            return .leading
        }
    }
    
    private var frameAlignment: Alignment {
        switch style {
        case .center:
            return .center
        case .default, .code:
            return .leading
        }
    }
}

#Preview {
    VStack(spacing: 20) {
        SlideView(slide: Slide(
            id: "1",
            title: "Welcome to Toboggan",
            body: "This is a presentation system built with Rust and SwiftUI",
            kind: .cover,
            style: .center,
            notes: "Remember to speak clearly and maintain eye contact"
        ))
        
        SlideView(slide: Slide(
            id: "2",
            title: "Code Example",
            body: "func hello() {\\n    print(\\"Hello, World!\\")\\n}",
            kind: .standard,
            style: .code,
            notes: nil
        ))
    }
    .padding()
}