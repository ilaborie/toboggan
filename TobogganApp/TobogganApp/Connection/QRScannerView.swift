//
//  QRScannerView.swift
//  TobogganApp
//

import AVFoundation
import SwiftUI

/// A QR reader, built on `AVCaptureMetadataOutput` rather than VisionKit's
/// `DataScannerViewController`.
///
/// Both are available at this deployment target. This one is chosen because the
/// job is "read one URL and dismiss": it has no ML dependency, no
/// `isSupported`/`isAvailable` pair to negotiate, and it does not bring its own
/// full-screen guidance UI along with it.
struct QRScannerView: UIViewControllerRepresentable {
    /// Called with the decoded string. The scanner stops before this runs, so it
    /// fires once.
    let onScan: (String) -> Void
    let onFailure: (String) -> Void

    func makeUIViewController(context: Context) -> QRScannerController {
        let controller = QRScannerController()
        controller.onScan = onScan
        controller.onFailure = onFailure
        return controller
    }

    func updateUIViewController(_ uiViewController: QRScannerController, context: Context) {}
}

/// The capture session behind ``QRScannerView``.
final class QRScannerController: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
    var onScan: ((String) -> Void)?
    var onFailure: ((String) -> Void)?

    private let session = AVCaptureSession()
    private var preview: AVCaptureVideoPreviewLayer?

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        configure()
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        preview?.frame = view.bounds
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        guard !session.isRunning else { return }
        // Starting the session blocks; the docs are explicit that it belongs off
        // the main thread.
        let session = session
        DispatchQueue.global(qos: .userInitiated).async { session.startRunning() }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        guard session.isRunning else { return }
        let session = session
        DispatchQueue.global(qos: .userInitiated).async { session.stopRunning() }
    }

    func metadataOutput(
        _ output: AVCaptureMetadataOutput,
        didOutput metadataObjects: [AVMetadataObject],
        from connection: AVCaptureConnection
    ) {
        guard let object = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
              object.type == .qr,
              let value = object.stringValue
        else {
            return
        }
        session.stopRunning()
        AppLog.shared.log(.ui, .info, "Scanned a QR code")
        onScan?(value)
    }

    private func configure() {
        guard let device = AVCaptureDevice.default(for: .video) else {
            fail("This device has no camera.")
            return
        }
        do {
            let input = try AVCaptureDeviceInput(device: device)
            guard session.canAddInput(input) else {
                fail("The camera could not be opened.")
                return
            }
            session.addInput(input)
        } catch {
            fail("The camera could not be opened: \(error.localizedDescription)")
            return
        }

        let output = AVCaptureMetadataOutput()
        guard session.canAddOutput(output) else {
            fail("The camera could not scan codes.")
            return
        }
        session.addOutput(output)
        output.setMetadataObjectsDelegate(self, queue: .main)
        // Set only after the output is attached: available types are empty
        // before that, and assigning .qr too early throws.
        output.metadataObjectTypes = [.qr]

        let preview = AVCaptureVideoPreviewLayer(session: session)
        preview.videoGravity = .resizeAspectFill
        preview.frame = view.bounds
        view.layer.addSublayer(preview)
        self.preview = preview
    }

    private func fail(_ message: String) {
        AppLog.shared.log(.ui, .error, message)
        onFailure?(message)
    }
}
