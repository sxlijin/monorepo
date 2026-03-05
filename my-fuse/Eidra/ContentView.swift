import SwiftUI
import FileProvider

struct ContentView: View {
    @State private var domainStatus = "Checking..."
    @State private var storageExists = false

    private let storageRoot = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent(".eidra-storage")

    var body: some View {
        VStack(spacing: 20) {
            Text("Eidra")
                .font(.largeTitle.bold())

            GroupBox("Status") {
                VStack(alignment: .leading, spacing: 8) {
                    Label(domainStatus, systemImage: domainStatus.contains("Active") ? "checkmark.circle.fill" : "hourglass")
                    Label(
                        "Storage: \(storageRoot.path)",
                        systemImage: storageExists ? "folder.fill" : "folder"
                    )
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(4)
            }

            Button("Refresh") { checkStatus() }
        }
        .padding(40)
        .frame(minWidth: 400, minHeight: 200)
        .onAppear { checkStatus() }
    }

    private func checkStatus() {
        storageExists = FileManager.default.fileExists(atPath: storageRoot.path)

        NSFileProviderManager.getDomainsWithCompletionHandler { domains, error in
            if let error {
                domainStatus = "Error: \(error.localizedDescription)"
            } else if domains.contains(where: { $0.identifier.rawValue == "com.roundcolors.eidra.domain" }) {
                domainStatus = "Active"
            } else {
                domainStatus = "Domain not registered"
            }
        }
    }
}
