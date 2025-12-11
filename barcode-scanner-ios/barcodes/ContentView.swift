//
//  ContentView.swift
//  barcodes
//
//  Created by Sam on 12/10/25.
//

import SwiftData
import SwiftUI

struct ContentView: View {
    var body: some View {
        NavigationStack {
            HomepageView()
        }
    }
}

#Preview {
    ContentView()
        .modelContainer(for: SavedBarcode.self, inMemory: true)
}
