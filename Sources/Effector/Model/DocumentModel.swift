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

/// Fits the (processed) image into a canvas of another aspect ratio, adding borders.
struct CanvasSpec: Codable, Hashable {
    var enabled = true
    var ratio = "4:5"            // see CanvasSpec.ratios, or "custom"
    var customW = 4.0
    var customH = 5.0
    var fill = "color"           // color, blur, gradient
    var color = RGBA(hex: "#FFFFFF")
    var color2 = RGBA(hex: "#E9E6DF")
    var padding = 4.0            // minimum border, % of the image's long side
    var offsetX = 0.0            // -100…100: where the image sits in the free space
    var offsetY = 0.0
    var radius = 0.0             // corner radius of the photo, % of its short side
    var shadow = 0.0             // 0…100
    var blur = 60.0              // background blur for "blur" fill

    static let ratios: [(id: String, label: String, w: Double, h: Double, group: String)] = [
        ("4:5", "4:5 · Instagram Portrait", 4, 5, "Social"), ("1:1", "1:1 · Square", 1, 1, "Social"),
        ("9:16", "9:16 · Story / Reels", 9, 16, "Social"), ("3:4", "3:4 · Instagram (new)", 3, 4, "Social"),
        ("1.91:1", "1.91:1 · Landscape Post", 1.91, 1, "Social"), ("16:9", "16:9 · Widescreen", 16, 9, "Screen"),
        ("4:3", "4:3", 4, 3, "Screen"), ("21:9", "21:9 · Ultrawide", 21, 9, "Screen"),
        ("2:3", "2:3 · Print Portrait", 2, 3, "Print"), ("3:2", "3:2 · Print Landscape", 3, 2, "Print"),
        ("5:7", "5:7 · Print", 5, 7, "Print"), ("8:10", "8:10 · Print", 8, 10, "Print"), ("A4", "A4 · Paper", 210, 297, "Print"),
        ("custom", "Custom…", 0, 0, "Custom"),
    ]

    var aspect: Double {
        if ratio == "custom" { return max(0.05, customW) / max(0.05, customH) }
        guard let r = CanvasSpec.ratios.first(where: { $0.id == ratio }), r.h > 0 else { return 0.8 }
        return r.w / r.h
    }

    /// Canvas size and the image rect inside it, for an image of w×h pixels.
    func layout(_ w: Double, _ h: Double) -> (size: CGSize, image: CGRect) {
        let a = aspect
        let m = padding / 100 * max(w, h)
        var W = max(w + 2 * m, (h + 2 * m) * a)
        var H = W / a
        if H < h + 2 * m { H = h + 2 * m; W = H * a }
        W = W.rounded(); H = H.rounded()
        let fx = (W - w) / 2, fy = (H - h) / 2
        let x = fx + fx * offsetX / 100, y = fy + fy * offsetY / 100
        return (CGSize(width: W, height: H), CGRect(x: x.rounded(), y: y.rounded(), width: w, height: h))
    }
}

/// The editable state of a document. Value type so undo is a snapshot.
struct DocumentState: Codable, Hashable {
    var layers: [LayerNode] = []
    var crop: CropRect = .full
    /// Aspect-ratio canvas with borders (nil = off). Optional so older projects still open.
    var canvas: CanvasSpec? = nil

    var activeCanvas: CanvasSpec? { canvas?.enabled == true ? canvas : nil }
}

struct ProjectManifest: Codable {
    var version = 1
    var app = "EFFECTOR"
    var originalFileName: String
    var state: DocumentState
}
