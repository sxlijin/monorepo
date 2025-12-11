import SwiftData
import SwiftUI

struct HomepageView: View {
    @Binding var selectedBarcodeID: PersistentIdentifier?
    @Query(sort: \SavedBarcode.createdAt, order: .reverse) private var barcodes: [SavedBarcode]
    @State private var showingAddSheet = false

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
                    NavigationLink(
                        tag: barcode.persistentModelID,
                        selection: $selectedBarcodeID
                    ) {
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
                Button {
                    showingAddSheet = true
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .sheet(isPresented: $showingAddSheet) {
            NavigationStack {
                AddBarcodeView()
            }
            .presentationDetents([.medium, .large])
            .presentationDragIndicator(.visible)
        }
    }
}
