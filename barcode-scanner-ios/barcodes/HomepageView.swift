import SwiftData
import SwiftUI

struct HomepageView: View {
    @Query(sort: \SavedBarcode.createdAt, order: .reverse) private var barcodes: [SavedBarcode]

    var body: some View {
        Group {
            if barcodes.isEmpty {
                ContentUnavailableView(
                    "No barcodes yet",
                    systemImage: "barcode.viewfinder",
                    description: Text("Add a barcode with the + button.")
                )
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
