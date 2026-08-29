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
    /// Called with the decoded string, exactly once — see `hasScanned`.
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

    /// `stopRunning` prevents *future* delivery but does not cancel batches
    /// already dispatched to the main queue, so the stop alone does not make
    /// `onScan` fire once.
    private var hasScanned = false

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black
        observeRuntimeFailures()
        requestAccessThenConfigure()
    }

    /// Asks for the camera before touching it.
    ///
    /// Without this the refusal was completely silent: with access denied,
    /// `AVCaptureDevice.default` still returns a device, `AVCaptureDeviceInput`
    /// still constructs, `canAddInput` is still true and `startRunning`
    /// succeeds — the session simply never delivers a frame. The presenter got a
    /// black sheet, no message, no log line, and no way forward, on the app's
    /// primary path for getting connected.
    private func requestAccessThenConfigure() {
        switch AVCaptureDevice.authorizationStatus(for: .video) {
        case .authorized:
            configure()
        case .notDetermined:
            AVCaptureDevice.requestAccess(for: .video) { [weak self] granted in
                DispatchQueue.main.async {
                    guard let self else { return }
                    guard granted else {
                        self.fail(
                            "Camera access was refused. Turn it on in Settings › Toboggan, "
                                + "or type the address by hand."
                        )
                        return
                    }
                    self.configure()
                    let session = self.session
                    DispatchQueue.global(qos: .userInitiated).async { session.startRunning() }
                }
            }
        case .denied:
            fail("Camera access is off for Toboggan. Turn it on in Settings › Toboggan, or type the address by hand.")
        case .restricted:
            fail("Camera access is restricted on this device. Type the server address by hand instead.")
        @unknown default:
            fail("Camera access could not be determined. Type the server address by hand instead.")
        }
    }

    /// A session can also die *after* it starts — the camera claimed by another
    /// app, a phone call, a Control Center capture. Left unobserved that is the
    /// same dead end as a refusal: a frozen frame and no explanation.
    private func observeRuntimeFailures() {
        NotificationCenter.default.addObserver(
            forName: .AVCaptureSessionRuntimeError,
            object: session,
            queue: .main
        ) { [weak self] note in
            let error = note.userInfo?[AVCaptureSessionErrorKey] as? Error
            let reason = error?.localizedDescription ?? "unknown error"
            self?.fail("The camera stopped: \(reason). Try again, or type the address by hand.")
        }
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        preview?.frame = view.bounds
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        // Nothing to start when access was refused: `configure` never ran, so
        // the session has no input.
        guard !session.isRunning, !session.inputs.isEmpty else { return }
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
        guard !hasScanned,
              let object = metadataObjects.first as? AVMetadataMachineReadableCodeObject,
              object.type == .qr,
              let value = object.stringValue
        else {
            return
        }
        hasScanned = true
        // Off the main thread, for the same reason `startRunning` is: the call
        // blocks. The flag above is what makes this fire once, so the stop does
        // not have to be synchronous.
        let session = session
        DispatchQueue.global(qos: .userInitiated).async { session.stopRunning() }
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

    deinit {
        NotificationCenter.default.removeObserver(self)
    }
}
