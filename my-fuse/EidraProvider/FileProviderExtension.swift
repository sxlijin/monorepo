import FileProvider
import UniformTypeIdentifiers
import os.log

class FileProviderExtension: NSObject, NSFileProviderReplicatedExtension, NSFileProviderEnumerating {
    private let logger = Logger(subsystem: "com.roundcolors.eidra.provider", category: "extension")
    private let domain: NSFileProviderDomain
    let storageRoot: URL

    required init(domain: NSFileProviderDomain) {
        self.domain = domain
        let home = FileManager.default.homeDirectoryForCurrentUser
        self.storageRoot = home.appendingPathComponent(".eidra-storage")
        super.init()

        // Ensure the storage directory exists.
        try? FileManager.default.createDirectory(at: storageRoot, withIntermediateDirectories: true)
        logger.info("Eidra storage root: \(self.storageRoot.path)")
    }

    func invalidate() {}

    // MARK: - Enumeration

    func enumerator(
        for containerItemIdentifier: NSFileProviderItemIdentifier,
        request: NSFileProviderRequest
    ) throws -> any NSFileProviderEnumerator {
        let containerPath: String
        switch containerItemIdentifier {
        case .rootContainer:
            containerPath = ""
        case .workingSet:
            // Working set: return an enumerator for the full root for now.
            containerPath = ""
        default:
            containerPath = containerItemIdentifier.rawValue
        }
        return FileProviderEnumerator(storageRoot: storageRoot, containerPath: containerPath)
    }

    // MARK: - Item Lookup

    func item(
        for identifier: NSFileProviderItemIdentifier,
        request: NSFileProviderRequest,
        completionHandler: @escaping (NSFileProviderItem?, Error?) -> Void
    ) -> Progress {
        let relativePath: String
        switch identifier {
        case .rootContainer:
            relativePath = ""
        default:
            relativePath = identifier.rawValue
        }

        let fileURL: URL
        if relativePath.isEmpty {
            fileURL = storageRoot
        } else {
            fileURL = storageRoot.appendingPathComponent(relativePath)
        }

        guard FileManager.default.fileExists(atPath: fileURL.path) else {
            completionHandler(nil, NSFileProviderError(.noSuchItem))
            return Progress()
        }

        let parentPath = (relativePath as NSString).deletingLastPathComponent
        let item = FileProviderItem.from(
            fileURL: fileURL,
            relativePath: relativePath,
            parentPath: parentPath.isEmpty && relativePath.contains("/") ? "" : (parentPath.isEmpty ? nil : parentPath)
        )
        completionHandler(item, nil)
        return Progress()
    }

    // MARK: - Fetch Contents

    func fetchContents(
        for itemIdentifier: NSFileProviderItemIdentifier,
        version requestedVersion: NSFileProviderItemVersion?,
        request: NSFileProviderRequest,
        completionHandler: @escaping (URL?, NSFileProviderItem?, Error?) -> Void
    ) -> Progress {
        let relativePath = itemIdentifier.rawValue
        let fileURL = storageRoot.appendingPathComponent(relativePath)

        guard FileManager.default.fileExists(atPath: fileURL.path) else {
            completionHandler(nil, nil, NSFileProviderError(.noSuchItem))
            return Progress()
        }

        let parentPath = (relativePath as NSString).deletingLastPathComponent
        let item = FileProviderItem.from(
            fileURL: fileURL,
            relativePath: relativePath,
            parentPath: parentPath.isEmpty ? nil : parentPath
        )
        completionHandler(fileURL, item, nil)
        return Progress()
    }

    // MARK: - Create Item

    func createItem(
        basedOn itemTemplate: NSFileProviderItem,
        fields: NSFileProviderItemFields,
        contents url: URL?,
        options: NSFileProviderCreateItemOptions = [],
        request: NSFileProviderRequest,
        completionHandler: @escaping (NSFileProviderItem?, NSFileProviderItemFields, Bool, Error?) -> Void
    ) -> Progress {
        let parentPath: String
        switch itemTemplate.parentItemIdentifier {
        case .rootContainer:
            parentPath = ""
        default:
            parentPath = itemTemplate.parentItemIdentifier.rawValue
        }

        let relativePath = parentPath.isEmpty
            ? itemTemplate.filename
            : "\(parentPath)/\(itemTemplate.filename)"
        let targetURL = storageRoot.appendingPathComponent(relativePath)

        let fm = FileManager.default
        do {
            if itemTemplate.contentType == .folder {
                try fm.createDirectory(at: targetURL, withIntermediateDirectories: true)
            } else if let sourceURL = url {
                // Ensure parent directory exists.
                let parentURL = targetURL.deletingLastPathComponent()
                try fm.createDirectory(at: parentURL, withIntermediateDirectories: true)
                if fm.fileExists(atPath: targetURL.path) {
                    try fm.removeItem(at: targetURL)
                }
                try fm.copyItem(at: sourceURL, to: targetURL)
            }

            let item = FileProviderItem.from(
                fileURL: targetURL,
                relativePath: relativePath,
                parentPath: parentPath.isEmpty ? nil : parentPath
            )
            completionHandler(item, [], false, nil)
        } catch {
            logger.error("createItem failed: \(error.localizedDescription)")
            completionHandler(nil, [], false, error)
        }
        return Progress()
    }

    // MARK: - Modify Item

    func modifyItem(
        _ item: NSFileProviderItem,
        baseVersion: NSFileProviderItemVersion,
        changedFields: NSFileProviderItemFields,
        contents newContents: URL?,
        options: NSFileProviderModifyItemOptions = [],
        request: NSFileProviderRequest,
        completionHandler: @escaping (NSFileProviderItem?, NSFileProviderItemFields, Bool, Error?) -> Void
    ) -> Progress {
        let relativePath = item.itemIdentifier.rawValue
        let targetURL = storageRoot.appendingPathComponent(relativePath)
        let fm = FileManager.default

        do {
            // Handle renames.
            if changedFields.contains(.filename) || changedFields.contains(.parentItemIdentifier) {
                let newParentPath: String
                switch item.parentItemIdentifier {
                case .rootContainer:
                    newParentPath = ""
                default:
                    newParentPath = item.parentItemIdentifier.rawValue
                }
                let newRelativePath = newParentPath.isEmpty
                    ? item.filename
                    : "\(newParentPath)/\(item.filename)"
                let newURL = storageRoot.appendingPathComponent(newRelativePath)

                if targetURL != newURL {
                    let parentURL = newURL.deletingLastPathComponent()
                    try fm.createDirectory(at: parentURL, withIntermediateDirectories: true)
                    try fm.moveItem(at: targetURL, to: newURL)

                    // Return item with new identifier.
                    let updatedItem = FileProviderItem.from(
                        fileURL: newURL,
                        relativePath: newRelativePath,
                        parentPath: newParentPath.isEmpty ? nil : newParentPath
                    )
                    completionHandler(updatedItem, [], false, nil)
                    return Progress()
                }
            }

            // Handle content updates.
            if changedFields.contains(.contents), let sourceURL = newContents {
                if fm.fileExists(atPath: targetURL.path) {
                    try fm.removeItem(at: targetURL)
                }
                try fm.copyItem(at: sourceURL, to: targetURL)
            }

            let parentPath = (relativePath as NSString).deletingLastPathComponent
            let updatedItem = FileProviderItem.from(
                fileURL: targetURL,
                relativePath: relativePath,
                parentPath: parentPath.isEmpty ? nil : parentPath
            )
            completionHandler(updatedItem, [], false, nil)
        } catch {
            logger.error("modifyItem failed: \(error.localizedDescription)")
            completionHandler(nil, [], false, error)
        }
        return Progress()
    }

    // MARK: - Delete Item

    func deleteItem(
        identifier: NSFileProviderItemIdentifier,
        baseVersion: NSFileProviderItemVersion,
        options: NSFileProviderDeleteItemOptions = [],
        request: NSFileProviderRequest,
        completionHandler: @escaping (Error?) -> Void
    ) -> Progress {
        let relativePath = identifier.rawValue
        let targetURL = storageRoot.appendingPathComponent(relativePath)

        do {
            if FileManager.default.fileExists(atPath: targetURL.path) {
                try FileManager.default.removeItem(at: targetURL)
            }
            completionHandler(nil)
        } catch {
            logger.error("deleteItem failed: \(error.localizedDescription)")
            completionHandler(error)
        }
        return Progress()
    }
}
