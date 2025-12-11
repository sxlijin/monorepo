//
//  ContentView.swift
//  barcodes
//
//  Created by Sam on 12/10/25.
//

import SwiftData
import SwiftUI
import UIKit

struct ContentView: View {
    @State private var selectedBarcodeID: PersistentIdentifier?
    @State private var previousBrightness: CGFloat?
    @Environment(\.scenePhase) private var scenePhase

    var body: some View {
        NavigationStack {
            HomepageView(selectedBarcodeID: $selectedBarcodeID)
        }
        .onChange(of: selectedBarcodeID) { newValue in
            if newValue != nil {
                captureAndBoostBrightness()
            } else {
                restoreBrightnessIfNeeded()
            }
        }
        .onChange(of: scenePhase) { phase in
            if phase == .active {
                if selectedBarcodeID != nil {
                    captureAndBoostBrightness()
                }
            } else {
                restoreBrightnessIfNeeded()
            }
        }
    }

    private func captureAndBoostBrightness() {
        if previousBrightness == nil {
            previousBrightness = UIScreen.main.brightness
        }
        UIScreen.main.brightness = 1.0
    }

    private func restoreBrightnessIfNeeded() {
        guard let previousBrightness else { return }
        UIScreen.main.brightness = previousBrightness
        self.previousBrightness = nil
    }
}

#Preview {
    ContentView()
        .modelContainer(for: SavedBarcode.self, inMemory: true)
}
