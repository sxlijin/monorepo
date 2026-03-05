import FileProvider
import os.log

class FileProviderEnumerator: NSObject, NSFileProviderEnumerator {
    private let logger = Logger(subsystem: "com.roundcolors.eidra.provider", category: "enumerator")
    private let storageRoot: URL
    private let containerPath: String // relative path within storage root, empty for root

    init(storageRoot: URL, containerPath: String) {
        self.storageRoot = storageRoot
        self.containerPath = containerPath
    }

    func invalidate() {}

    func enumerateItems(
        for observer: any NSFileProviderEnumerationObserver,
        startingAt page: NSFileProviderPage
    ) {
        let containerURL: URL
        if containerPath.isEmpty {
            containerURL = storageRoot
        } else {
            containerURL = storageRoot.appendingPathComponent(containerPath)
        }

        logger.info("Enumerating items at: \(containerURL.path)")

        let fm = FileManager.default
        guard let contents = try? fm.contentsOfDirectory(
            at: containerURL,
            includingPropertiesForKeys: [.isDirectoryKey, .fileSizeKey, .creationDateKey, .contentModificationDateKey],
            options: [.skipsHiddenFiles]
        ) else {
            logger.error("Failed to enumerate directory: \(containerURL.path)")
            observer.finishEnumerating(upTo: nil)
            return
        }

        let items: [FileProviderItem] = contents.map { url in
            let childName = url.lastPathComponent
            let relativePath = containerPath.isEmpty ? childName : "\(containerPath)/\(childName)"
            let parentPath: String? = containerPath.isEmpty ? nil : containerPath
            return FileProviderItem.from(fileURL: url, relativePath: relativePath, parentPath: parentPath)
        }

        observer.didEnumerate(items)
        observer.finishEnumerating(upTo: nil)
    }

    func enumerateChanges(
        for observer: any NSFileProviderChangeObserver,
        from anchor: NSFileProviderSyncAnchor
    ) {
        // For the POC, report no incremental changes — the system will re-enumerate.
        let currentAnchor = currentSyncAnchor()
        observer.finishEnumeratingChanges(upTo: currentAnchor, moreComing: false)
    }

    func currentSyncAnchor(completionHandler: @escaping (NSFileProviderSyncAnchor?) -> Void) {
        completionHandler(currentSyncAnchor())
    }

    private func currentSyncAnchor() -> NSFileProviderSyncAnchor {
        let now = Date().timeIntervalSince1970
        let data = withUnsafeBytes(of: now) { Data($0) }
        return NSFileProviderSyncAnchor(data)
    }
}
