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
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .modelContainer(for: SavedBarcode.self)
    }
}
