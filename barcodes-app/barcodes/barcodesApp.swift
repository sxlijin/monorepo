//
//  barcodesApp.swift
//  barcodes
//
//  Created by Sam on 12/10/25.
//

import SwiftData
import SwiftUI

@main
struct barcodesApp: App {
    let container: ModelContainer

    init() {
        do {
            container = try ModelContainer(for: SavedBarcode.self)
        } catch {
            fatalError("Failed to create ModelContainer: \(error)")
        }
        #if DEBUG
        // Populate deterministic sample data for App Store screenshots.
        // Enabled only when launched with `-seedSampleData`; never runs in release builds.
        if ProcessInfo.processInfo.arguments.contains("-seedSampleData") {
            seedSampleData()
        }
        #endif
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .modelContainer(container)
    }

    #if DEBUG
    @MainActor
    private func seedSampleData() {
        let context = container.mainContext
        try? context.delete(model: SavedBarcode.self)
        let samples: [(title: String, payload: String, daysAgo: Double)] = [
            ("Gym Membership", "GYM-4827-1190", 2),
            ("Library Card", "9780201379624", 9),
            ("Coffee Loyalty", "CAFE0036000291", 16),
            ("Conference Badge", "ATTENDEE-2026-5512", 23),
            ("Warehouse Club", "049000028911", 41),
        ]
        for sample in samples {
            context.insert(
                SavedBarcode(
                    title: sample.title,
                    payload: sample.payload,
                    createdAt: Date(timeIntervalSinceNow: -sample.daysAgo * 86_400)
                )
            )
        }
        try? context.save()
    }
    #endif
}
