import Foundation
import CoreGraphics

/// Normalized crop rect (0..1 in source image space, origin top-left).
struct CropRect: Codable, Hashable {
    var x: Double = 0
    var y: Double = 0
    var w: Double = 1
    var h: Double = 1

    static let full = CropRect()
    var isFull: Bool { x <= 0.0001 && y <= 0.0001 && w >= 0.9999 && h >= 0.9999 }

    func pixelRect(width: Int, height: Int) -> CGRect {
        CGRect(x: (x * Double(width)).rounded(), y: (y * Double(height)).rounded(),
               width: max(1, (w * Double(width)).rounded()), height: max(1, (h * Double(height)).rounded()))
    }
}

/// The editable state of a document. Value type so undo is a snapshot.
struct DocumentState: Codable, Hashable {
    var layers: [LayerNode] = []
    var crop: CropRect = .full
}

struct ProjectManifest: Codable {
    var version = 1
    var app = "EFFECTOR"
    var originalFileName: String
    var state: DocumentState
}
