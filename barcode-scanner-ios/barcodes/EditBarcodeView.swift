import Observation
import SwiftData
import SwiftUI

struct EditBarcodeView: View {
    @Bindable var barcode: SavedBarcode
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext

    @State private var title: String
    @State private var payload: String

    var onDelete: () -> Void = {}

    init(barcode: SavedBarcode, onDelete: @escaping () -> Void = {}) {
        self._barcode = Bindable(wrappedValue: barcode)
        self._title = State(initialValue: barcode.title)
        self._payload = State(initialValue: barcode.payload)
        self.onDelete = onDelete
    }

    var body: some View {
        BarcodeFormView(
            title: $title,
            payload: $payload,
            mode: .edit,
            onSave: saveChanges,
            onDelete: deleteBarcode
        )
        .navigationTitle("Edit barcode")
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button("Cancel") { dismiss() }
            }
        }
    }

    private func saveChanges() {
        let trimmedPayload = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedPayload.isEmpty else { return }
        barcode.title = title.trimmingCharacters(in: .whitespacesAndNewlines)
        barcode.payload = trimmedPayload
        barcode.lastUpdated = Date()
        dismiss()
    }

    private func deleteBarcode() {
        modelContext.delete(barcode)
        dismiss()
        onDelete()
    }
}
