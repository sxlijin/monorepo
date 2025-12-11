//
//  ContentView.swift
//  barcodes
//
//  Created by Sam on 12/10/25.
//

import SwiftData
import SwiftUI
import Vision
import VisionKit

struct ContentView: View {
    @Query(sort: \SavedBarcode.createdAt, order: .reverse) private var barcodes: [SavedBarcode]
    @Environment(\.modelContext) private var modelContext

    var body: some View {
        NavigationStack {
            Group {
                if barcodes.isEmpty {
                    ContentUnavailableView("No barcodes yet", systemImage: "barcode.viewfinder", description: Text("Add a barcode with the + button."))
                        .padding(.horizontal)
                } else {
                    List(barcodes) { barcode in
                        NavigationLink {
                            BarcodeDetailView(barcode: barcode)
                        } label: {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(barcode.title.isEmpty ? "Untitled" : barcode.title)
                                    .font(.headline)
                                Text(barcode.payload)
                                    .font(.subheadline)
                                    .foregroundStyle(.secondary)
                                Text(barcode.createdAt, format: .dateTime)
                                    .font(.caption)
                                    .foregroundStyle(.tertiary)
                            }
                        }
                    }
                    .listStyle(.insetGrouped)
                }
            }
            .navigationTitle("Barcodes")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    NavigationLink {
                        AddBarcodeView()
                    } label: {
                        Image(systemName: "plus")
                    }
                }
            }
        }
    }
}

private struct BarcodeDetailView: View {
    let barcode: SavedBarcode

    private static let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text(barcode.title.isEmpty ? "Untitled" : barcode.title)
                .font(.largeTitle)
                .bold()
            VStack(alignment: .leading, spacing: 8) {
                Text("Value")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(barcode.payload)
                    .font(.title3.monospaced())
                    .textSelection(.enabled)
            }
            VStack(alignment: .leading, spacing: 4) {
                Text("Created")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Text(BarcodeDetailView.dateFormatter.string(from: barcode.createdAt))
                    .font(.subheadline)
            }
            Spacer()
        }
        .padding()
        .navigationTitle("Barcode")
        .navigationBarTitleDisplayMode(.inline)
    }
}

private struct AddBarcodeView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext

    @State private var title = ""
    @State private var payload = ""
    @State private var showingScanner = false
    @State private var scannerUnavailableMessage: String?

    private var isSaveDisabled: Bool { payload.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

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
                Button("Save") {
                    saveBarcode()
                }
                .disabled(isSaveDisabled)
            }
        }
        .navigationTitle("Add barcode")
        .onAppear {
            updateScannerAvailability()
        }
        .sheet(isPresented: $showingScanner) {
            BarcodeScannerView { value in
                payload = value
                if title.isEmpty {
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

    private func saveBarcode() {
        let trimmedPayload = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedPayload.isEmpty else { return }
        let barcode = SavedBarcode(
            title: title.trimmingCharacters(in: .whitespacesAndNewlines),
            payload: trimmedPayload,
            createdAt: Date()
        )
        modelContext.insert(barcode)
        dismiss()
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

private struct BarcodeScannerView: UIViewControllerRepresentable {
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

#Preview {
    ContentView()
        .modelContainer(for: SavedBarcode.self, inMemory: true)
}
