import Observation
import SwiftData
import SwiftUI

struct EditBarcodeView: View {
    @Bindable var barcode: SavedBarcode
    @Environment(\.dismiss) private var dismiss
    @Environment(\.modelContext) private var modelContext

    @State private var title: String
    @State private var payload: String
    @State private var showingDiscardAlert = false

    private let originalTitle: String
    private let originalPayload: String

    var onDelete: () -> Void = {}

    init(barcode: SavedBarcode, onDelete: @escaping () -> Void = {}) {
        self._barcode = Bindable(wrappedValue: barcode)
        self._title = State(initialValue: barcode.title)
        self._payload = State(initialValue: barcode.payload)
        self.originalTitle = barcode.title.trimmingCharacters(in: .whitespacesAndNewlines)
        self.originalPayload = barcode.payload.trimmingCharacters(in: .whitespacesAndNewlines)
        self.onDelete = onDelete
    }

    var body: some View {
        BarcodeFormView(
            title: $title,
            payload: $payload,
            mode: .edit,
            onSave: saveChanges,
            onDelete: deleteBarcode,
            createdAt: barcode.createdAt,
            lastUpdated: barcode.lastUpdated,
            includeInlineSaveButton: false
        )
        .navigationTitle("Edit barcode")
        .toolbar {
            ToolbarItem(placement: .cancellationAction) {
                Button("Cancel") { attemptDismiss() }
            }
            ToolbarItem(placement: .navigationBarTrailing) {
                Button("Save") {
                    saveChanges()
                }
                .disabled(isSaveDisabled)
            }
        }
        .interactiveDismissDisabled(hasUnsavedChanges)
        .alert("Discard changes?", isPresented: $showingDiscardAlert) {
            Button("Keep Editing", role: .cancel) {}
            Button("Discard", role: .destructive) {
                dismiss()
            }
        } message: {
            Text("You have unsaved changes.")
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

    private var isSaveDisabled: Bool {
        payload.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private func attemptDismiss() {
        if hasUnsavedChanges {
            showingDiscardAlert = true
        } else {
            dismiss()
        }
    }

    private var hasUnsavedChanges: Bool {
        title.trimmingCharacters(in: .whitespacesAndNewlines) != originalTitle ||
        payload.trimmingCharacters(in: .whitespacesAndNewlines) != originalPayload
    }

    private func deleteBarcode() {
        modelContext.delete(barcode)
        dismiss()
        onDelete()
    }
}
