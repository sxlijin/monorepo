import CoreImage
import CoreImage.CIFilterBuiltins
import SwiftUI

struct BarcodeDetailView: View {
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
            if let image = barcodeImage(for: barcode.payload) {
                image
                    .resizable()
                    .interpolation(.none)
                    .scaledToFit()
                    .frame(maxWidth: 280)
                    .padding(.vertical)
                    .accessibilityLabel("Barcode image")
            } else {
                Text("Unable to generate barcode image")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
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

    private func barcodeImage(for value: String) -> Image? {
        guard !value.isEmpty else { return nil }
        let filter = CIFilter.code128BarcodeGenerator()
        filter.message = Data(value.utf8)
        filter.quietSpace = 7

        guard
            let outputImage = filter.outputImage?
                .transformed(by: CGAffineTransform(scaleX: 3, y: 3)),
            let cgImage = CIContext().createCGImage(outputImage, from: outputImage.extent)
        else {
            return nil
        }

        return Image(decorative: cgImage, scale: 1, orientation: .up)
    }
}
