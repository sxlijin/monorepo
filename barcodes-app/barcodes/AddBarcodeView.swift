import SwiftData
import SwiftUI

struct AddBarcodeView: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext

    @State private var title = ""
    @State private var payload = ""

    var body: some View {
        BarcodeFormView(
            title: $title,
            payload: $payload,
            mode: .add,
            onSave: saveBarcode,
            includeInlineSaveButton: false
        )
        .navigationTitle("Add barcode")
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button("Cancel") { dismiss() }
            }
            ToolbarItem(placement: .navigationBarTrailing) {
                Button("Save") {
                    saveBarcode()
                }
                .disabled(isSaveDisabled)
            }
        }
    }

    private var isSaveDisabled: Bool {
        payload.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func saveBarcode() {
        let trimmedPayload = payload.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedPayload.isEmpty else { return }
        let barcode = SavedBarcode(
            title: title.trimmingCharacters(in: .whitespacesAndNewlines),
            payload: trimmedPayload,
            createdAt: Date(),
            lastUpdated: Date()
        )
        modelContext.insert(barcode)
        dismiss()
    }
}
