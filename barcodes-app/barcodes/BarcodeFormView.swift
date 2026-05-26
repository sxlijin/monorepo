import SwiftUI
import UIKit
import Vision
import VisionKit

struct BarcodeFormView: View {
    enum Mode {
        case add
        case edit
    }

    @Binding var title: String
    @Binding var payload: String

    var mode: Mode
    var onSave: () -> Void
    var onDelete: (() -> Void)? = nil
    var createdAt: Date? = nil
    var lastUpdated: Date? = nil
    var includeInlineSaveButton: Bool = true

    @State private var showingScanner = false
    @State private var scannerUnavailableMessage: String?

    private var isSaveDisabled: Bool {
        payload.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private var saveButtonTitle: String {
        mode == .add ? "Save" : "Save changes"
    }

    private static let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()

    var body: some View {
        Form {
            Section("Details") {
                TextField("Title", text: $title)
                TextField("Barcode text or URL", text: $payload, axis: .vertical)
                    .lineLimit(1...3)
            }

            Section {
                Button {
                    showingScanner = true
                } label: {
                    Label("Take photo to scan", systemImage: "camera.viewfinder")
                }
                .disabled(!isScannerAvailable)
                if let scannerUnavailableMessage {
                    Text(scannerUnavailableMessage)
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                }
            } footer: {
                Text("Uses the camera to detect barcodes and fill the value.")
            }

            if includeInlineSaveButton {
                Section {
                    Button(saveButtonTitle) {
                        onSave()
                    }
                    .disabled(isSaveDisabled)
                }
            }

            if mode == .edit, let onDelete {
                Section {
                    Button("Delete barcode", role: .destructive) {
                        onDelete()
                    }
                }
            }

            if mode == .edit, let createdAt, let lastUpdated {
                Section("Metadata") {
                    LabeledContent("Created") {
                        Text(BarcodeFormView.dateFormatter.string(from: createdAt))
                            .foregroundStyle(.secondary)
                    }
                    LabeledContent("Modified") {
                        Text(BarcodeFormView.dateFormatter.string(from: lastUpdated))
                            .foregroundStyle(.secondary)
                    }
                }
            }
        }
        .onAppear {
            updateScannerAvailability()
        }
        .sheet(isPresented: $showingScanner) {
            BarcodeScannerView { value in
                payload = value
                showingScanner = false
            } onError: { error in
                scannerUnavailableMessage = error.localizedDescription
                showingScanner = false
            }
        }
    }

    private var isScannerAvailable: Bool {
        scannerUnavailableMessage == nil
    }

    private func updateScannerAvailability() {
        guard DataScannerViewController.isSupported else {
            scannerUnavailableMessage = "Barcode scanning is not supported on this device."
            return
        }
        guard DataScannerViewController.isAvailable else {
            scannerUnavailableMessage = "Camera access is unavailable (simulator or restricted)."
            return
        }
        scannerUnavailableMessage = nil
    }
}

struct BarcodeScannerView: UIViewControllerRepresentable {
    typealias UIViewControllerType = DataScannerViewController

    var onCapture: (String) -> Void
    var onError: (Error) -> Void

    private let topMaskTag = 9001
    private let bottomMaskTag = 9002
    private static let middleScanRegion = CGRect(x: 0, y: 1.0 / 3.0, width: 1.0, height: 1.0 / 3.0)

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeUIViewController(context: Context) -> DataScannerViewController {
        let symbologies: [VNBarcodeSymbology] = [
            .qr,
            .ean8, .ean13, .code39, .code93, .code128,
            .upce, .itf14, .dataMatrix, .pdf417, .aztec
        ]

        let controller = DataScannerViewController(
            recognizedDataTypes: [.barcode(symbologies: symbologies)],
            qualityLevel: .balanced,
            recognizesMultipleItems: false,
            isHighFrameRateTrackingEnabled: false,
            isPinchToZoomEnabled: true,
            isGuidanceEnabled: true,
            isHighlightingEnabled: true
        )
        controller.delegate = context.coordinator
        configureMaskOverlays(for: controller)
        return controller
    }

    func updateUIViewController(_ uiViewController: DataScannerViewController, context: Context) {
        configureMaskOverlays(for: uiViewController)
        guard !context.coordinator.isScanning else { return }
        do {
            try uiViewController.startScanning()
            context.coordinator.isScanning = true
        } catch {
            onError(error)
        }
    }

    private func configureMaskOverlays(for controller: DataScannerViewController) {
        let container = controller.overlayContainerView
        addMaskIfNeeded(to: container, tag: topMaskTag, anchorToTop: true)
        addMaskIfNeeded(to: container, tag: bottomMaskTag, anchorToTop: false)
    }

    private func addMaskIfNeeded(to container: UIView, tag: Int, anchorToTop: Bool) {
        guard container.viewWithTag(tag) == nil else { return }
        let maskView = UIView()
        maskView.tag = tag
        maskView.isUserInteractionEnabled = false
        maskView.backgroundColor = UIColor.black.withAlphaComponent(0.6)
        maskView.translatesAutoresizingMaskIntoConstraints = false
        container.addSubview(maskView)

        let verticalConstraint = anchorToTop
            ? maskView.topAnchor.constraint(equalTo: container.topAnchor)
            : maskView.bottomAnchor.constraint(equalTo: container.bottomAnchor)

        NSLayoutConstraint.activate([
            verticalConstraint,
            maskView.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            maskView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            maskView.heightAnchor.constraint(equalTo: container.heightAnchor, multiplier: 1.0 / 3.0)
        ])
    }

    final class Coordinator: NSObject, DataScannerViewControllerDelegate {
        var parent: BarcodeScannerView
        var isScanning = false
        private var hasCaptured = false

        init(parent: BarcodeScannerView) {
            self.parent = parent
        }

        func dataScanner(_ dataScanner: DataScannerViewController, didAdd addedItems: [RecognizedItem], allItems: [RecognizedItem]) {
            guard !hasCaptured else { return }
            if let value = extractedPayload(from: addedItems) {
                hasCaptured = true
                parent.onCapture(value)
                dataScanner.stopScanning()
            }
        }

        func dataScanner(_ dataScanner: DataScannerViewController, didTapOn item: RecognizedItem) {
            guard !hasCaptured else { return }
            if let value = extractedPayload(from: [item]) {
                hasCaptured = true
                parent.onCapture(value)
                dataScanner.stopScanning()
            }
        }

        func dataScanner(_ dataScanner: DataScannerViewController, becameUnavailableWithError error: any Error) {
            parent.onError(error)
            dataScanner.stopScanning()
        }

        private func extractedPayload(from items: [RecognizedItem]) -> String? {
            for item in items {
                if case let .barcode(barcode) = item,
                   isWithinScanRegion(barcode.observation.boundingBox),
                   let value = barcode.payloadStringValue {
                    return value
                }
            }
            return nil
        }

        private func isWithinScanRegion(_ boundingBox: CGRect) -> Bool {
            guard boundingBox.width > 0, boundingBox.height > 0 else { return false }
            let centerPoint = CGPoint(x: boundingBox.midX, y: boundingBox.midY)
            return BarcodeScannerView.middleScanRegion.contains(centerPoint)
        }
    }
}
