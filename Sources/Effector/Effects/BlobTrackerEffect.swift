import Foundation
import Metal
import Vision
import CoreImage
import AppKit

enum BlobShaders {
    static let source = """
    // base, overlay (premultiplied rgba8), mask (r = invert region). u: invert amount, dim outside
    kernel void k_blob(KIN, texture2d<float> base [[texture(1)]], texture2d<float> ov [[texture(2)]], texture2d<float> mask [[texture(3)]]) {
        GUARD
        float4 b = base.read(gid);
        float2 uv = UV;
        float m = mask.sample(sLin, uv).a;
        float3 c = b.rgb;
        c = mix(c, 1.0 - c, m * u[0]);
        c *= 1.0 - u[1] * (1.0 - m);
        float4 t = ov.sample(sLin, uv);
        c = c * (1.0 - t.a) + t.rgb;
        o.write(float4(clamp(c, 0.0, 1.0), b.a), gid);
    }
    """
}

struct Blob {
    var rect: CGRect      // normalized 0..1, top-left origin
    var area: Double      // normalized
    var center: CGPoint { CGPoint(x: rect.midX, y: rect.midY) }
}

enum BlobDetector {
    /// Detection on a small RGBA float buffer (w×h).
    static func components(_ px: [Float], _ w: Int, _ h: Int, test: (Int) -> Bool, minArea: Double, maxArea: Double) -> [Blob] {
        var label = [Int32](repeating: 0, count: w * h)
        var blobs: [Blob] = []
        var stack: [Int] = []
        var next: Int32 = 1
        for start in 0..<(w * h) where label[start] == 0 && test(start) {
            var minX = w, minY = h, maxX = 0, maxY = 0, count = 0
            stack.append(start); label[start] = next
            while let i = stack.popLast() {
                let x = i % w, y = i / w
                count += 1
                minX = min(minX, x); maxX = max(maxX, x); minY = min(minY, y); maxY = max(maxY, y)
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let nx = x + dx, ny = y + dy
                    guard nx >= 0, ny >= 0, nx < w, ny < h else { continue }
                    let j = ny * w + nx
                    if label[j] == 0 && test(j) { label[j] = next; stack.append(j) }
                }
            }
            next += 1
            let area = Double(count) / Double(w * h)
            if area >= minArea && area <= maxArea {
                blobs.append(Blob(rect: CGRect(x: Double(minX) / Double(w), y: Double(minY) / Double(h),
                                               width: Double(maxX - minX + 1) / Double(w), height: Double(maxY - minY + 1) / Double(h)),
                                  area: area))
            }
        }
        return blobs
    }

    static func vision(_ tex: MTLTexture, mode: String, minArea: Double, maxArea: Double) -> [Blob] {
        guard let cg = ImageExport.cgImage(tex) else { return [] }
        let handler = VNImageRequestHandler(cgImage: cg, options: [:])
        var out: [Blob] = []
        func add(_ bb: CGRect) {
            // Vision: normalized, bottom-left origin
            let r = CGRect(x: bb.minX, y: 1 - bb.maxY, width: bb.width, height: bb.height)
            let a = Double(r.width * r.height)
            if a >= minArea && a <= maxArea { out.append(Blob(rect: r, area: a)) }
        }
        if mode == "saliency" {
            let req = VNGenerateObjectnessBasedSaliencyImageRequest()
            try? handler.perform([req])
            for o in (req.results?.first?.salientObjects ?? []) { add(o.boundingBox) }
            let req2 = VNGenerateAttentionBasedSaliencyImageRequest()
            try? handler.perform([req2])
            for o in (req2.results?.first?.salientObjects ?? []) { add(o.boundingBox) }
        } else {
            let req = VNDetectContoursRequest()
            req.contrastAdjustment = 1.5
            req.detectsDarkOnLight = true
            req.maximumImageDimension = 512
            try? handler.perform([req])
            if let obs = req.results?.first {
                for c in obs.topLevelContours { add(c.normalizedPath.boundingBox) }
                if out.count < 6 {
                    for i in 0..<min(obs.contourCount, 400) {
                        if let c = try? obs.contour(at: i) { add(c.normalizedPath.boundingBox) }
                    }
                }
            }
        }
        return out
    }
}

struct BlobTrackerEffect: Effect {
    let id = "blob"
    let name = "Blob Tracker"
    let category = EffectCategory.overlay
    var symbol: String { "viewfinder" }
    var summary: String { "TouchDesigner-style tracking boxes, labels and connections." }
    let params: [ParamDef] = [
        .choice("detect", "Detect By", [("brightness", "Brightness"), ("darkness", "Darkness"), ("color", "Color"),
                                        ("contours", "Contours (Vision)"), ("saliency", "Objects (Vision)")], "brightness", section: "Detection"),
        .slider("threshold", "Threshold", 0, 100, 65, unit: "%", section: "Detection",
                visible: whenAny("detect", ["brightness", "darkness"])),
        .color("target", "Target Color", RGBA(hex: "#E03020"), section: "Detection", visible: when("detect", "color")),
        .slider("tolerance", "Tolerance", 1, 100, 25, unit: "%", section: "Detection", visible: when("detect", "color")),
        .slider("maxBlobs", "Max Blobs", 1, 60, 14, step: 1, section: "Detection"),
        .slider("minSize", "Min Size", 0, 20, 0.3, unit: "%", curve: 3, section: "Detection"),
        .slider("maxSize", "Max Size", 1, 100, 40, unit: "%", curve: 2, section: "Detection"),
        .slider("detail", "Detail", 40, 400, 160, step: 10, section: "Detection"),
        .choice("shape", "Marker", [("box", "Box"), ("corners", "Corners"), ("circle", "Circle"), ("cross", "Crosshair"),
                                    ("brackets", "Brackets")], "box", section: "Style"),
        .segmented("boxMode", "Box Size", [("fit", "Fit Blob"), ("fixed", "Fixed")], "fit", section: "Style"),
        .slider("boxScale", "Box Scale", 10, 200, 100, step: 1, unit: "%", section: "Style", visible: whenNot("boxMode", "fixed")),
        .slider("boxSize", "Box Size", 4, 200, 30, step: 1, curve: 1.8, section: "Style", visible: when("boxMode", "fixed")),
        .slider("padding", "Padding", -20, 40, 4, step: 0.5, section: "Style", visible: whenNot("boxMode", "fixed")),
        .color("color", "Color", RGBA(hex: "#FFFFFF"), section: "Style"),
        .slider("lineWidth", "Line Width", 0.25, 8, 1.6, step: 0.05, curve: 1.8, section: "Style"),
        .toggle("centerDot", "Center Dot", true, section: "Style"),
        .slider("fill", "Fill", 0, 100, 0, unit: "%", section: "Style"),
        .slider("invertInside", "Invert Inside", 0, 100, 0, unit: "%", section: "Style"),
        .slider("dimOutside", "Dim Outside", 0, 100, 0, unit: "%", section: "Style"),
        .choice("label", "Label", [("none", "None"), ("id", "ID"), ("coords", "Coordinates"), ("size", "Size"),
                                   ("hex", "Hex ID"), ("custom", "Custom Text")], "id", section: "Labels"),
        ParamDef(key: "labelText", label: "Text", kind: .text(def: "TARGET"), section: "Labels", visible: when("label", "custom")),
        .slider("labelSize", "Label Size", 4, 60, 16, step: 0.5, curve: 1.5, section: "Labels", visible: whenNot("label", "none")),
        .toggle("labelBG", "Label Background", true, section: "Labels", visible: whenNot("label", "none")),
        .choice("connect", "Connections", [("none", "None"), ("sequence", "Sequence"), ("nearest", "Nearest"), ("center", "To Center"),
                                           ("all", "All")], "nearest", section: "Connections"),
        .slider("connectWidth", "Line Width", 0.25, 6, 1.0, step: 0.05, curve: 1.8, section: "Connections", visible: whenNot("connect", "none")),
        .toggle("dashed", "Dashed", false, section: "Connections", visible: whenNot("connect", "none")),
        .slider("connectOpacity", "Opacity", 0, 100, 70, unit: "%", section: "Connections", visible: whenNot("connect", "none")),
    ]

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let mode = p.choice("detect", "brightness")
        let minA = p.num("minSize", 0.3) / 100, maxA = p.num("maxSize", 40) / 100
        var blobs: [Blob]
        if mode == "contours" || mode == "saliency" {
            let f = min(1, 768 / Double(max(input.width, input.height)))
            let small = ctx.resize(input, max(1, Int(Double(input.width) * f)), max(1, Int(Double(input.height) * f)))
            ctx.flush()
            blobs = BlobDetector.vision(small, mode: mode, minArea: minA, maxArea: mode == "saliency" ? 1 : maxA)
        } else {
            let detail = p.num("detail", 160)
            let s = detail / Double(max(input.width, input.height))
            let w = max(8, Int(Double(input.width) * s)), h = max(8, Int(Double(input.height) * s))
            let small = ctx.resize(input, w, h)
            let px = ctx.readFloats(small)
            let th = Float(p.num("threshold", 65) / 100)
            let target = p.color("target", RGBA(hex: "#E03020"))
            let tol = Float(p.num("tolerance", 25) / 100)
            let test: (Int) -> Bool
            switch mode {
            case "darkness":
                test = { i in 0.2126 * px[i * 4] + 0.7152 * px[i * 4 + 1] + 0.0722 * px[i * 4 + 2] < 1 - th }
            case "color":
                let tr = Float(target.r), tg = Float(target.g), tb = Float(target.b)
                test = { i in
                    let dr = px[i * 4] - tr, dg = px[i * 4 + 1] - tg, db = px[i * 4 + 2] - tb
                    return (dr * dr + dg * dg + db * db).squareRoot() < tol * 1.2
                }
            default:
                test = { i in 0.2126 * px[i * 4] + 0.7152 * px[i * 4 + 1] + 0.0722 * px[i * 4 + 2] > th }
            }
            blobs = BlobDetector.components(px, w, h, test: test, minArea: minA, maxArea: maxA)
        }
        blobs.sort { $0.area > $1.area }
        blobs = Array(blobs.prefix(Int(p.num("maxBlobs", 14))))
        // stable reading order for IDs / sequence: left-to-right, top-to-bottom
        blobs.sort { ($0.center.y * 4).rounded() == ($1.center.y * 4).rounded() ? $0.center.x < $1.center.x : $0.center.y < $1.center.y }

        let W = CGFloat(input.width), H = CGFloat(input.height)
        let ov = OverlayCanvas(input.width, input.height)
        let mask = OverlayCanvas(input.width, input.height)
        let color = p.color("color", .white)
        let lw = CGFloat(max(0.5, ctx.px(p.num("lineWidth", 1.6))))
        let pad = CGFloat(ctx.px(p.num("padding", 4)))
        let fixed = p.choice("boxMode", "fit") == "fixed"
        let bs = CGFloat(p.num("boxScale", 100) / 100)
        let fs = CGFloat(ctx.px(p.num("boxSize", 30)))
        let rects: [CGRect] = blobs.map { b in
            let cx = b.rect.midX * W, cy = b.rect.midY * H
            if fixed { return CGRect(x: cx - fs / 2, y: cy - fs / 2, width: fs, height: fs) }
            let w = max(2, b.rect.width * W * bs + pad * 2), h = max(2, b.rect.height * H * bs + pad * 2)
            return CGRect(x: cx - w / 2, y: cy - h / 2, width: w, height: h)
        }
        let g = ov.ctx
        // connections (under markers)
        let conn = p.choice("connect", "nearest")
        if conn != "none" && rects.count > 1 {
            g.setStrokeColor(ov.cg(color, alpha: p.num("connectOpacity", 70) / 100))
            g.setLineWidth(CGFloat(max(0.5, ctx.px(p.num("connectWidth", 1.0)))))
            if p.bool("dashed") { g.setLineDash(phase: 0, lengths: [lw * 4, lw * 3]) }
            let cs = rects.map { CGPoint(x: $0.midX, y: $0.midY) }
            switch conn {
            case "sequence":
                g.addLines(between: cs)
            case "center":
                let c = CGPoint(x: W / 2, y: H / 2)
                for pt in cs { g.move(to: c); g.addLine(to: pt) }
            case "all":
                for i in 0..<cs.count { for j in (i + 1)..<cs.count { g.move(to: cs[i]); g.addLine(to: cs[j]) } }
            default:
                for (i, a) in cs.enumerated() {
                    var best = -1; var bd = CGFloat.infinity
                    for (j, b) in cs.enumerated() where j != i {
                        let d = hypot(a.x - b.x, a.y - b.y)
                        if d < bd { bd = d; best = j }
                    }
                    if best >= 0 { g.move(to: a); g.addLine(to: cs[best]) }
                }
            }
            g.strokePath()
            g.setLineDash(phase: 0, lengths: [])
        }
        // markers
        g.setStrokeColor(ov.cg(color))
        g.setFillColor(ov.cg(color, alpha: p.num("fill") / 100))
        g.setLineWidth(lw)
        let shape = p.choice("shape", "box")
        for r in rects {
            mask.ctx.setFillColor(CGColor(gray: 1, alpha: 1))
            mask.ctx.fill(r)
            if p.num("fill") > 0 { g.fill(r) }
            switch shape {
            case "corners", "brackets":
                let L = min(r.width, r.height) * (shape == "corners" ? 0.25 : 0.15)
                let pts: [(CGPoint, CGPoint, CGPoint)] = [
                    (CGPoint(x: r.minX, y: r.minY + L), CGPoint(x: r.minX, y: r.minY), CGPoint(x: r.minX + L, y: r.minY)),
                    (CGPoint(x: r.maxX - L, y: r.minY), CGPoint(x: r.maxX, y: r.minY), CGPoint(x: r.maxX, y: r.minY + L)),
                    (CGPoint(x: r.maxX, y: r.maxY - L), CGPoint(x: r.maxX, y: r.maxY), CGPoint(x: r.maxX - L, y: r.maxY)),
                    (CGPoint(x: r.minX + L, y: r.maxY), CGPoint(x: r.minX, y: r.maxY), CGPoint(x: r.minX, y: r.maxY - L)),
                ]
                if shape == "brackets" {
                    g.addLines(between: [CGPoint(x: r.minX + L, y: r.minY), CGPoint(x: r.minX, y: r.minY), CGPoint(x: r.minX, y: r.maxY), CGPoint(x: r.minX + L, y: r.maxY)])
                    g.addLines(between: [CGPoint(x: r.maxX - L, y: r.minY), CGPoint(x: r.maxX, y: r.minY), CGPoint(x: r.maxX, y: r.maxY), CGPoint(x: r.maxX - L, y: r.maxY)])
                } else {
                    for (a, b, c) in pts { g.addLines(between: [a, b, c]) }
                }
                g.strokePath()
            case "circle":
                let d = max(r.width, r.height)
                g.strokeEllipse(in: CGRect(x: r.midX - d / 2, y: r.midY - d / 2, width: d, height: d))
            case "cross":
                let L = max(r.width, r.height) * 0.6
                g.move(to: CGPoint(x: r.midX - L, y: r.midY)); g.addLine(to: CGPoint(x: r.midX + L, y: r.midY))
                g.move(to: CGPoint(x: r.midX, y: r.midY - L)); g.addLine(to: CGPoint(x: r.midX, y: r.midY + L))
                g.strokePath()
                g.stroke(r.insetBy(dx: r.width * 0.25, dy: r.height * 0.25))
            default:
                g.stroke(r)
            }
            if p.bool("centerDot", true) {
                let d = lw * 2.5
                g.setFillColor(ov.cg(color))
                g.fillEllipse(in: CGRect(x: r.midX - d / 2, y: r.midY - d / 2, width: d, height: d))
                g.setFillColor(ov.cg(color, alpha: p.num("fill") / 100))
            }
        }
        // labels
        let labelMode = p.choice("label", "id")
        if labelMode != "none" {
            let size = CGFloat(ctx.px(p.num("labelSize", 16)))
            let inv = color.luma > 0.5 ? RGBA.black : RGBA.white
            for (i, r) in rects.enumerated() {
                let text: String
                switch labelMode {
                case "coords": text = String(format: "X%04d Y%04d", Int(r.midX / W * 1000), Int(r.midY / H * 1000))
                case "size": text = String(format: "%d×%d", Int(r.width / W * 1000), Int(r.height / H * 1000))
                case "hex": text = String(format: "0x%04X", (i * 7919 + Int(r.midX) * 31 + Int(r.midY)) & 0xFFFF)
                case "custom": text = p.text("labelText", "TARGET") + (rects.count > 1 ? " \(i + 1)" : "")
                default: text = String(format: "ID %02d", i + 1)
                }
                let bg = p.bool("labelBG", true)
                let y = max(0, r.minY - size * 1.35 - lw)
                ov.text(text, at: CGPoint(x: r.minX + (bg ? size * 0.25 : 0), y: y + size * 0.12), size: size,
                        color: bg ? inv : color, weight: .semibold, background: bg ? color : nil, padding: bg ? size * 0.2 : 0)
            }
        }
        return ctx.apply("k_blob", [input, ov.texture(), mask.texture()],
                         [p.f("invertInside") / 100, p.f("dimOutside") / 100 * 0.85])
    }
}
