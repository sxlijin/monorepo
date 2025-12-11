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
