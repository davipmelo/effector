import Foundation
import CoreGraphics
import UniformTypeIdentifiers

/// One open image in the workspace: its original file, its edits and its undo history.
final class DocSlot: ObservableObject, Identifiable {
    let id = UUID()
    let data: Data
    let fileName: String
    let typeHint: UTType?
    var fileURL: URL?
    /// Decoded pixels (released for images not used recently).
    var source: SourceImage?
    var state = DocumentState()
    var undo: [DocumentState] = []
    var redo: [DocumentState] = []
    var projectURL: URL?
    var isDirty = false
    var selectedID: UUID?
    var zoom: Double?
    var pan: CGSize = .zero
    var lastUsed = Date()
    /// File name inside the session folder.
    var sessionFile: String?
    @Published var thumb: CGImage?

    init(source: SourceImage, fileURL: URL?) {
        self.source = source
        self.data = source.data
        self.fileName = source.fileName
        self.typeHint = fileURL.flatMap { UTType(filenameExtension: $0.pathExtension) }
            ?? UTType(filenameExtension: (source.fileName as NSString).pathExtension)
        self.fileURL = fileURL
    }

    init(data: Data, fileName: String, typeHint: UTType?, fileURL: URL?) {
        self.data = data
        self.fileName = fileName
        self.typeHint = typeHint
        self.fileURL = fileURL
    }

    func loadSource() throws -> SourceImage {
        if let s = source { return s }
        let s = try SourceImage.load(data: data, fileName: fileName, typeHint: typeHint)
        source = s
        return s
    }
}
