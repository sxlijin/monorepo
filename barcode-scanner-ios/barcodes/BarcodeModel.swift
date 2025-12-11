//
//  BarcodeModel.swift
//  barcodes
//
//  Created by Sam on 12/10/25.
//

import Foundation
import SwiftData

@Model
final class SavedBarcode {
    var title: String
    var payload: String
    var createdAt: Date

    init(title: String, payload: String, createdAt: Date = .init()) {
        self.title = title
        self.payload = payload
        self.createdAt = createdAt
    }
}
