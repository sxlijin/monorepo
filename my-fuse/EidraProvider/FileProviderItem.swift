import FileProvider
import UniformTypeIdentifiers

class FileProviderItem: NSObject, NSFileProviderItem {
    let identifier: NSFileProviderItemIdentifier
    let parentIdentifier: NSFileProviderItemIdentifier
    let storedFilename: String
    let storedContentType: UTType
    let storedSize: Int64?
    let storedCreationDate: Date?
    let storedModificationDate: Date?
    let storedVersion: NSFileProviderItemVersion

    var itemIdentifier: NSFileProviderItemIdentifier { identifier }
    var parentItemIdentifier: NSFileProviderItemIdentifier { parentIdentifier }
    var filename: String { storedFilename }
    var contentType: UTType { storedContentType }
    var documentSize: NSNumber? { storedSize.map { NSNumber(value: $0) } }
    var creationDate: Date? { storedCreationDate }
    var contentModificationDate: Date? { storedModificationDate }
    var itemVersion: NSFileProviderItemVersion { storedVersion }

    init(
        identifier: NSFileProviderItemIdentifier,
        parentIdentifier: NSFileProviderItemIdentifier,
        filename: String,
        contentType: UTType,
        size: Int64? = nil,
        creationDate: Date? = nil,
        modificationDate: Date? = nil,
        version: NSFileProviderItemVersion
    ) {
        self.identifier = identifier
        self.parentIdentifier = parentIdentifier
        self.storedFilename = filename
        self.storedContentType = contentType
        self.storedSize = size
        self.storedCreationDate = creationDate
        self.storedModificationDate = modificationDate
        self.storedVersion = version
    }

    /// Build a FileProviderItem from a file URL relative to the storage root.
    /// `relativePath` is the slash-separated path from the storage root (e.g. "photos/cat.jpg").
    /// An empty relativePath means the root container itself.
    static func from(
        fileURL: URL,
        relativePath: String,
        parentPath: String?
    ) -> FileProviderItem {
        let fm = FileManager.default
        let attrs = try? fm.attributesOfItem(atPath: fileURL.path)
        let isDirectory = (attrs?[.type] as? FileAttributeType) == .typeDirectory

        let identifier: NSFileProviderItemIdentifier
        if relativePath.isEmpty {
            identifier = .rootContainer
        } else {
            identifier = NSFileProviderItemIdentifier(relativePath)
        }

        let parentIdentifier: NSFileProviderItemIdentifier
        if let parentPath, !parentPath.isEmpty {
            parentIdentifier = NSFileProviderItemIdentifier(parentPath)
        } else {
            parentIdentifier = .rootContainer
        }

        let contentType: UTType = isDirectory
            ? .folder
            : (UTType(filenameExtension: fileURL.pathExtension) ?? .data)

        let size: Int64? = isDirectory
            ? nil
            : (attrs?[.size] as? Int64)

        let creationDate = attrs?[.creationDate] as? Date
        let modificationDate = attrs?[.modificationDate] as? Date

        // Use modification date as a simple content version.
        let modTimeInterval = modificationDate?.timeIntervalSince1970 ?? 0
        let versionData = withUnsafeBytes(of: modTimeInterval) { Data($0) }
        let version = NSFileProviderItemVersion(
            contentVersion: versionData,
            metadataVersion: versionData
        )

        return FileProviderItem(
            identifier: identifier,
            parentIdentifier: parentIdentifier,
            filename: fileURL.lastPathComponent,
            contentType: contentType,
            size: size,
            creationDate: creationDate,
            modificationDate: modificationDate,
            version: version
        )
    }
}
