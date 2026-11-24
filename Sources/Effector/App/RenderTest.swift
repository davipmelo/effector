import Foundation
import AppKit
import ImageIO
import CoreText

/// Headless rendering used for automated checks:
/// Effector --render-test <image> <outdir> [effectID[:key=value,...] ...]
enum RenderTest {
    /// Consistency checks for every effect definition, look and factory preset.
    static func lint() {
        var problems: [String] = []
        for e in EffectRegistry.all {
            var keys = Set<String>()
            for d in e.params {
                if !keys.insert(d.key).inserted { problems.append("\(e.id): duplicate key \(d.key)") }
                switch d.kind {
                case .slider(let mn, let mx, let def, _, _, _):
                    if def < mn || def > mx { problems.append("\(e.id).\(d.key): default \(def) outside \(mn)…\(mx)") }
                case .choice(let o, let def), .segmented(let o, let def):
                    if !o.contains(where: { $0.id == def }) { problems.append("\(e.id).\(d.key): default \(def) not an option") }
                default: break
                }
            }
            for l in e.looks {
                for k in l.values.keys where !keys.contains(k) { problems.append("\(e.id) look \(l.id): unknown key \(k)") }
            }
        }
        func check(_ nodes: [LayerNode], _ where_: String) {
            for n in nodes {
                if n.isGroup { check(n.children, where_); continue }
                guard let e = EffectRegistry.effect(n.effectID) else { problems.append("\(where_): missing effect \(n.effectID)"); continue }
                let keys = Set(e.params.map(\.key))
                for k in n.params.values.keys where !keys.contains(k) && k != "_look" { problems.append("\(where_) / \(e.id): unknown key \(k)") }
            }
        }
        for p in FactoryPresets.all { check(p.layers, "preset \(p.name)") }
        print(problems.isEmpty ? "lint: OK" : problems.joined(separator: "\n"))
        print("effects: \(EffectRegistry.all.count), presets: \(FactoryPresets.all.count)")
    }

    static func run(_ args: [String]) {
        AppState.headless = true
        guard let i = args.firstIndex(of: "--render-test"), args.count > i + 2 else {
            print("usage: --render-test <image> <outdir> [effect[:k=v,...]]"); return
        }
        let input = URL(fileURLWithPath: args[i + 1])
        let outDir = URL(fileURLWithPath: args[i + 2])
        try? FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
        if args.contains("--lint") { lint(); return }
        if args.contains("--multidoc") {
            let app = AppState.shared
            app.viewportSize = CGSize(width: 800, height: 600)
            func spin(_ t: Double) { RunLoop.main.run(until: Date().addingTimeInterval(t)) }
            let a = try! SourceImage.load(url: URL(fileURLWithPath: args[i + 1]))
            app.openImage(a); spin(1.5)
            print("doc1 output:", app.output?.width ?? -1, app.output?.height ?? -1)
            let b = try! SourceImage.load(url: URL(fileURLWithPath: args[i + 2]))
            app.openImage(b); spin(1.5)
            print("doc2 output:", app.output?.width ?? -1, app.output?.height ?? -1, "source:", app.source?.width ?? -1)
            app.activateNext(-1); spin(1.5)
            print("back to doc1:", app.output?.width ?? -1, app.output?.height ?? -1, "source:", app.source?.width ?? -1)
            app.addEffect("film")
            app.edit { $0.canvas = CanvasSpec(ratio: "1:1", fill: "blur") }
            spin(1)
            print("doc1 output with canvas:", app.output?.width ?? -1, app.output?.height ?? -1)
            let other = Set(app.docs.map(\.id)).subtracting([app.activeID!])
            app.syncSettings(to: other, [.layers, .canvas])
            app.activateNext(1); spin(1.5)
            print("doc2 after sync: layers", app.state.layers.count, "canvas", app.state.canvas?.ratio ?? "-", "output", app.output?.width ?? -1, app.output?.height ?? -1, "dirty", app.isDirty)
            app.undo(); spin(1)
            print("doc2 after undo: layers", app.state.layers.count)
            return
        }
        if args.contains("--project-roundtrip") {
            let app = AppState.shared
            do {
                app.openImage(try SourceImage.load(url: input))
                app.state.layers = FactoryPresets.all.first { $0.name == "Surveillance" }!.layers
                app.state.crop = CropRect(x: 0.1, y: 0.1, w: 0.8, h: 0.6)
                let url = outDir.appendingPathComponent("test.effector")
                try app.writeProject(to: url)
                let saved = app.state
                app.openImage(try SourceImage.load(url: input))
                try app.openProject(url)
                print("roundtrip equal:", saved == app.state, "layers:", app.state.layers.count,
                      "files:", (try? FileManager.default.contentsOfDirectory(atPath: url.path)) ?? [])
            } catch { print("roundtrip error:", error) }
            return
        }
        let specs = Array(args.dropFirst(i + 3)).filter { !$0.hasPrefix("--") }
        do {
            let src = try SourceImage.load(url: input)
            let scale = args.contains("--full2x") ? 2.0 : (args.contains("--full") ? 1.0 : min(1, 1200 / Double(max(src.width, src.height))))
            let tex = src.makeTexture(crop: .full, scale: scale)
            let r = Renderer(useCache: false)
            let list = specs.isEmpty ? EffectRegistry.all.map { $0.id } : specs
            var sheet: [(String, CGImage)] = [("original", ImageExport.cgImage(tex)!)]
            for spec in list {
                // spec "a:k=v,k=v+b:k=v" -> stack (first = bottom)
                var layers: [LayerNode] = []
                if spec == "presets" {
                    for pr in FactoryPresets.all {
                        let out = r.render(layers: pr.layers, source: tex, sourceKey: 0)
                        if let cg = ImageExport.cgImage(out) { sheet.append((pr.name, cg)) }
                    }
                    continue
                }
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
                let out = r.render(layers: layers, source: tex, sourceKey: 0)
                let ms = Int(Date().timeIntervalSince(t0) * 1000)
                if args.contains("--check-neutral") {
                    let rc = RenderContext(scale: 1, source: out, isExport: true)
                    let f = rc.readFloats(out)
                    var maxDiff: Float = 0
                    for i in stride(from: 0, to: f.count, by: 4) {
                        maxDiff = max(maxDiff, abs(f[i] - f[i + 1]), abs(f[i + 1] - f[i + 2]))
                    }
                    print("neutral-check \(spec): max channel difference = \(maxDiff)")
                }
                let name = spec.replacingOccurrences(of: ":", with: "_").replacingOccurrences(of: ",", with: "_")
                    .replacingOccurrences(of: "=", with: "-").replacingOccurrences(of: "+", with: "__")
                let url = outDir.appendingPathComponent(String(name.prefix(120)) + ".jpg")
                try ImageExport.write(out, to: url, format: .jpg, quality: 0.85, keepAlpha: false)
                if !args.contains("--full") && !args.contains("--full2x"), let cg = ImageExport.cgImage(out) { sheet.append((spec, cg)) }
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
