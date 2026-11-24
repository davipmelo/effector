// Generates the EFFECTOR app icon: a dithered yellow "E" on black.
// usage: swift make_icon.swift <out.iconset dir>
import AppKit
import CoreGraphics
import CoreText

let outDir = CommandLine.arguments[1]
try? FileManager.default.createDirectory(atPath: outDir, withIntermediateDirectories: true)

func render(size S: Int) -> CGImage {
    let cs = CGColorSpace(name: CGColorSpace.sRGB)!
    let ctx = CGContext(data: nil, width: S, height: S, bitsPerComponent: 8, bytesPerRow: 0, space: cs,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    let s = CGFloat(S)
    // macOS icon grid: 824/1024 body with rounded corners
    let inset = s * 100 / 1024
    let body = CGRect(x: inset, y: inset, width: s - inset * 2, height: s - inset * 2)
    let radius = body.width * 0.225
    let path = CGPath(roundedRect: body, cornerWidth: radius, cornerHeight: radius, transform: nil)
    // shadow
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -s * 0.01), blur: s * 0.025, color: CGColor(gray: 0, alpha: 0.45))
    ctx.addPath(path); ctx.setFillColor(CGColor(gray: 0.05, alpha: 1)); ctx.fillPath()
    ctx.restoreGState()
    ctx.saveGState()
    ctx.addPath(path); ctx.clip()
    let grad = CGGradient(colorsSpace: cs, colors: [CGColor(gray: 0.13, alpha: 1), CGColor(gray: 0.03, alpha: 1)] as CFArray, locations: [0, 1])!
    ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: body.maxY), end: CGPoint(x: 0, y: body.minY), options: [])

    // Glyph mask for "E"
    let maskSize = 256
    let mctx = CGContext(data: nil, width: maskSize, height: maskSize, bitsPerComponent: 8, bytesPerRow: maskSize,
                         space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue)!
    mctx.setFillColor(gray: 0, alpha: 1); mctx.fill(CGRect(x: 0, y: 0, width: maskSize, height: maskSize))
    let font = NSFont.systemFont(ofSize: 250, weight: .black)
    let attr = NSAttributedString(string: "E", attributes: [.font: font, .foregroundColor: NSColor.white])
    let line = CTLineCreateWithAttributedString(attr)
    let b = CTLineGetImageBounds(line, mctx)
    mctx.textPosition = CGPoint(x: (CGFloat(maskSize) - b.width) / 2 - b.minX, y: (CGFloat(maskSize) - b.height) / 2 - b.minY)
    CTLineDraw(line, mctx)
    let mdata = mctx.data!.assumingMemoryBound(to: UInt8.self)

    // Dither: grid of cells inside the body, Bayer 4x4 against a vertical gradient
    let bayer: [[Double]] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]
    let cells = 24
    let gridRect = body.insetBy(dx: body.width * 0.1, dy: body.height * 0.1)
    let cw = gridRect.width / CGFloat(cells)
    for gy in 0..<cells {
        for gx in 0..<cells {
            // sample mask at cell center (mask y is bottom-up like CG)
            let mx = Int((CGFloat(gx) + 0.5) / CGFloat(cells) * CGFloat(maskSize))
            let my = Int((CGFloat(gy) + 0.5) / CGFloat(cells) * CGFloat(maskSize))
            let m = Double(mdata[my * maskSize + mx]) / 255
            if m < 0.5 { continue }
            // gradient: top solid, fading toward bottom-right
            let t = 0.3 + 1.0 * (Double(gy) / Double(cells - 1)) - 0.1 * (Double(gx) / Double(cells - 1))
            let threshold = (bayer[gy % 4][gx % 4] + 0.5) / 16
            if t < threshold { continue }
            let r = CGRect(x: gridRect.minX + CGFloat(gx) * cw, y: gridRect.minY + CGFloat(gy) * cw, width: cw, height: cw)
                .insetBy(dx: cw * 0.06, dy: cw * 0.06)
            ctx.setFillColor(CGColor(srgbRed: 1, green: 0.733, blue: 0, alpha: 1))
            ctx.fill(r)
        }
    }
    // subtle top highlight
    ctx.setStrokeColor(CGColor(gray: 1, alpha: 0.08)); ctx.setLineWidth(s * 0.004)
    ctx.addPath(path); ctx.strokePath()
    ctx.restoreGState()
    return ctx.makeImage()!
}

let sizes: [(String, Int)] = [("16x16", 16), ("16x16@2x", 32), ("32x32", 32), ("32x32@2x", 64), ("128x128", 128),
                              ("128x128@2x", 256), ("256x256", 256), ("256x256@2x", 512), ("512x512", 512), ("512x512@2x", 1024)]
for (name, px) in sizes {
    let img = render(size: px)
    let rep = NSBitmapImageRep(cgImage: img)
    try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "\(outDir)/icon_\(name).png"))
}
print("icon ok")
