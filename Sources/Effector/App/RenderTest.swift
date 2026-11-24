import Foundation
import AppKit
import ImageIO
import CoreText

/// Headless rendering used for automated checks:
/// Effector --render-test <image> <outdir> [effectID[:key=value,...] ...]
enum RenderTest {
    static func run(_ args: [String]) {
        guard let i = args.firstIndex(of: "--render-test"), args.count > i + 2 else {
            print("usage: --render-test <image> <outdir> [effect[:k=v,...]]"); return
        }
        let input = URL(fileURLWithPath: args[i + 1])
        let outDir = URL(fileURLWithPath: args[i + 2])
        try? FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
        let specs = Array(args.dropFirst(i + 3))
        do {
            let src = try SourceImage.load(url: input)
            let scale = min(1, 1200 / Double(max(src.width, src.height)))
            let tex = src.makeTexture(crop: .full, scale: scale)
            let r = Renderer(useCache: false)
            let list = specs.isEmpty ? EffectRegistry.all.map { $0.id } : specs
            var sheet: [(String, CGImage)] = [("original", ImageExport.cgImage(tex)!)]
            for spec in list {
                // spec "a:k=v,k=v+b:k=v" -> stack (first = bottom)
                var layers: [LayerNode] = []
                for part in spec.split(separator: "+") {
                    let comps = part.split(separator: ":", maxSplits: 1)
                    var node = LayerNode.effect(String(comps[0]))
                    if comps.count > 1 {
                        for kv in comps[1].split(separator: ",") {
                            let p = kv.split(separator: "=", maxSplits: 1)
                            guard p.count == 2 else { continue }
                            let k = String(p[0]), v = String(p[1])
                            if let d = Double(v) { node.params[k] = .number(d) }
                            else if v == "true" || v == "false" { node.params[k] = .bool(v == "true") }
                            else { node.params[k] = .choice(v) }
                        }
                    }
                    layers.insert(node, at: 0)
                }
                let t0 = Date()
                let out = r.render(layers: layers, source: tex, sourceKey: 0, scale: scale * 1.0)
                let ms = Int(Date().timeIntervalSince(t0) * 1000)
                let name = spec.replacingOccurrences(of: ":", with: "_").replacingOccurrences(of: ",", with: "_")
                    .replacingOccurrences(of: "=", with: "-").replacingOccurrences(of: "+", with: "__")
                let url = outDir.appendingPathComponent(String(name.prefix(120)) + ".jpg")
                try ImageExport.write(out, to: url, format: .jpg, quality: 0.85, keepAlpha: false)
                if let cg = ImageExport.cgImage(out) { sheet.append((spec, cg)) }
                print("ok \(spec) \(ms)ms -> \(url.lastPathComponent)")
            }
            writeSheet(sheet, to: outDir.appendingPathComponent("_sheet.jpg"))
        } catch {
            print("error: \(error)")
        }
    }

    /// Contact sheet: grid of results with labels.
    static func writeSheet(_ items: [(String, CGImage)], to url: URL) {
        guard let first = items.first?.1 else { return }
        let cols = min(4, items.count)
        let rows = (items.count + cols - 1) / cols
        let cw = 600, ch = Int(Double(cw) * Double(first.height) / Double(first.width))
        let labelH = 26
        let W = cols * cw, H = rows * (ch + labelH)
        let ctx = CGContext(data: nil, width: W, height: H, bitsPerComponent: 8, bytesPerRow: 0,
                            space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
        ctx.setFillColor(CGColor(gray: 0.12, alpha: 1)); ctx.fill(CGRect(x: 0, y: 0, width: W, height: H))
        for (i, (name, img)) in items.enumerated() {
            let c = i % cols, r = i / cols
            let y = H - (r + 1) * (ch + labelH)
            ctx.interpolationQuality = .high
            ctx.draw(img, in: CGRect(x: c * cw, y: y, width: cw, height: ch))
            let attr = NSAttributedString(string: name, attributes: [.font: NSFont.systemFont(ofSize: 15, weight: .medium), .foregroundColor: NSColor.white])
            let line = CTLineCreateWithAttributedString(attr)
            ctx.textPosition = CGPoint(x: c * cw + 8, y: y + ch + 7)
            CTLineDraw(line, ctx)
        }
        if let cg = ctx.makeImage(), let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.jpeg" as CFString, 1, nil) {
            CGImageDestinationAddImage(dest, cg, [kCGImageDestinationLossyCompressionQuality: 0.85] as CFDictionary)
            CGImageDestinationFinalize(dest)
        }
    }
}
