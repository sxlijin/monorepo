import SwiftUI
import FileProvider
import os.log

@main
struct EidraApp: App {
    private let logger = Logger(subsystem: "com.roundcolors.eidra", category: "app")

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }

    init() {
        if CommandLine.arguments.contains("--remove-domain") {
            removeDomain()
        } else {
            registerDomain()
        }
    }

    private func registerDomain() {
        let domain = NSFileProviderDomain(
            identifier: NSFileProviderDomainIdentifier("com.roundcolors.eidra.domain"),
            displayName: "Eidra"
        )

        NSFileProviderManager.add(domain) { error in
            if let error {
                self.logger.error("Failed to register domain: \(error.localizedDescription)")
            } else {
                self.logger.info("Eidra domain registered successfully")
            }
        }
    }

    private func removeDomain() {
        let domain = NSFileProviderDomain(
            identifier: NSFileProviderDomainIdentifier("com.roundcolors.eidra.domain"),
            displayName: "Eidra"
        )

        NSFileProviderManager.remove(domain) { error in
            if let error {
                self.logger.error("Failed to remove domain: \(error.localizedDescription)")
                fputs("Failed to remove domain: \(error.localizedDescription)\n", stderr)
            } else {
                self.logger.info("Eidra domain removed")
                print("Eidra domain removed")
            }
            exit(error == nil ? 0 : 1)
        }
    }
}
