import SwiftUI
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

    @State private var showingScanner = false
    @State private var scannerUnavailableMessage: String?

    private var isSaveDisabled: Bool {
        payload.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private var saveButtonTitle: String {
        mode == .add ? "Save" : "Save changes"
    }

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

            Section {
                Button(saveButtonTitle) {
                    onSave()
                }
                .disabled(isSaveDisabled)
            }

            if mode == .edit, let onDelete {
                Section {
                    Button("Delete barcode", role: .destructive) {
                        onDelete()
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
                if title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    title = "Barcode"
                }
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
        return controller
    }

    func updateUIViewController(_ uiViewController: DataScannerViewController, context: Context) {
        guard !context.coordinator.isScanning else { return }
        do {
            try uiViewController.startScanning()
            context.coordinator.isScanning = true
        } catch {
            onError(error)
        }
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
                if case let .barcode(barcode) = item, let value = barcode.payloadStringValue {
                    return value
                }
            }
            return nil
        }
    }
}
