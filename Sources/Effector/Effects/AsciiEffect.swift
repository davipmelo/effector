import Foundation
import AppKit
import CoreText
import Metal

enum AsciiShaders {
    static let source = """
    // u: cellW, cellH, glyphCount, mode(0 mono,1 palette,2 original), fg rgb(4..6), bg rgb(7..9),
    //    invert, contrast, brightness, atlasCellW(px), atlasW
    kernel void k_ascii(KIN, texture2d<float> grid [[texture(1)]], texture2d<float> atlas [[texture(2)]]) {
        GUARD
        float2 p = float2(gid) + 0.5;
        float2 cs = float2(u[0], u[1]);
        float2 cellF = p / cs;
        uint2 cell = uint2(floor(cellF));
        cell = min(cell, uint2(grid.get_width() - 1, grid.get_height() - 1));
        float2 local = fract(cellF);
        float4 c = grid.read(cell);
        float l = neutralGray(clamp(c.rgb, 0.0, 1.0));
        l = clamp((l - 0.5) * (1.0 + u[11] / 100.0) + 0.5 + u[12] / 100.0, 0.0, 1.0);
        if (u[10] > 0.5) l = 1.0 - l;
        float n = max(1.0, u[2]);
        float idx = min(n - 1.0, floor(l * n));
        float ax = (idx + local.x) / n;
        float g = atlas.sample(sLin, float2(ax, local.y)).r;
        float3 fg = float3(u[4], u[5], u[6]);
        float3 bg = float3(u[7], u[8], u[9]);
        int mode = int(u[3]);
        if (mode == 1) {
            int pc = int(ex[0]);
            int pi = clamp(int(floor(l * float(pc))), 0, pc - 1);
            fg = float3(ex[1 + pi * 3], ex[2 + pi * 3], ex[3 + pi * 3]);
        } else if (mode == 2) {
            fg = clamp(c.rgb * 1.15, 0.0, 1.0);
        }
        o.write(float4(mix(bg, fg, g), c.a), gid);
    }
    """
}

/// Builds and caches glyph atlases sorted by ink density.
final class GlyphAtlas {
    static let shared = GlyphAtlas()
    private var cache: [String: (MTLTexture, Int)] = [:]
    private let lock = NSLock()
    static let cellW = 40, cellH = 72

    func atlas(chars: String, bold: Bool) -> (MTLTexture, Int) {
        let key = chars + (bold ? "|b" : "|r")
        lock.lock(); defer { lock.unlock() }
        if let c = cache[key] { return c }
        if cache.count > 32 { cache.removeAll() }   // custom character sets change while typing
        let glyphs = Array(chars).map(String.init)
        let gw = GlyphAtlas.cellW, gh = GlyphAtlas.cellH
        // render each glyph separately to measure density
        var rendered: [(density: Double, pixels: [UInt8])] = []
        let font = NSFont.monospacedSystemFont(ofSize: CGFloat(gh) * 0.78, weight: bold ? .heavy : .medium)
        for g in glyphs {
            let ctx = CGContext(data: nil, width: gw, height: gh, bitsPerComponent: 8, bytesPerRow: gw,
                                space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue)!
            ctx.setFillColor(gray: 0, alpha: 1)
            ctx.fill(CGRect(x: 0, y: 0, width: gw, height: gh))
            let attr = NSAttributedString(string: g, attributes: [.font: font, .foregroundColor: NSColor.white])
            let line = CTLineCreateWithAttributedString(attr)
            let wdt = CTLineGetTypographicBounds(line, nil, nil, nil)
            // block elements fill the whole cell: stretch them
            let isBlock = g.unicodeScalars.first.map { (0x2580...0x259F).contains($0.value) } ?? false
            if isBlock {
                ctx.scaleBy(x: CGFloat(gw) / CGFloat(max(1, wdt)), y: CGFloat(gh) / (font.ascender - font.descender))
                ctx.textPosition = CGPoint(x: 0, y: -font.descender)
            } else {
                ctx.textPosition = CGPoint(x: (CGFloat(gw) - CGFloat(wdt)) / 2, y: CGFloat(gh) * 0.2)
            }
            CTLineDraw(line, ctx)
            let raw = ctx.data!.assumingMemoryBound(to: UInt8.self)
            let buf = Array(UnsafeBufferPointer(start: raw, count: gw * gh))
            let density = Double(buf.reduce(0) { $0 + Int($1) }) / Double(gw * gh * 255)
            // bitmap memory starts at the top row, matching texture v=0
            rendered.append((density, buf))
        }
        rendered.sort { $0.density < $1.density }
        let n = rendered.count
        var atlas = [UInt8](repeating: 0, count: n * gw * gh)
        for (i, r) in rendered.enumerated() {
            for y in 0..<gh { for x in 0..<gw { atlas[y * n * gw + i * gw + x] = r.pixels[y * gw + x] } }
        }
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .r8Unorm, width: n * gw, height: gh, mipmapped: false)
        d.usage = [.shaderRead]; d.storageMode = .shared
        let t = GPU.shared.device.makeTexture(descriptor: d)!
        t.replace(region: MTLRegionMake2D(0, 0, n * gw, gh), mipmapLevel: 0, withBytes: atlas, bytesPerRow: n * gw)
        cache[key] = (t, n)
        return (t, n)
    }
}

struct AsciiEffect: Effect {
    let id = "ascii"
    let name = "ASCII"
    let category = EffectCategory.stylize
    var symbol: String { "textformat.characters" }
    var summary: String { "Rebuilds the image from characters." }

    static let charsets: [(String, String, String)] = [
        ("standard", "Standard", " .:-=+*#%@"),
        ("detailed", "Detailed", " .'`^\",:;Il!i><~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$"),
        ("blocks", "Blocks", " ░▒▓█"),
        ("dots", "Dots", " ·•●"),
        ("binary", "Binary", " 01"),
        ("hex", "Hex", " 0123456789ABCDEF"),
        ("slashes", "Slashes", " /\\X"),
        ("braille", "Braille", " ⠁⠃⠇⠏⠟⠿⡿⣿"),
        ("geometric", "Geometric", " ·○◎●■"),
        ("letters", "Letters", " ilEFFECTOR"),
        ("matrix", "Matrix", " ｱｲｳｴｵｶｷｸｹｺ"),
        ("emoji", "Shapes", " ▫◽◻⬜"),
        ("custom", "Custom…", ""),
    ]

    let params: [ParamDef] = [
        .choice("charset", "Characters", AsciiEffect.charsets.map { ($0.0, $0.1) }, "standard", section: "Characters"),
        ParamDef(key: "customChars", label: "Your Characters (dark → light order is automatic)", kind: .text(def: " .oO@"),
                 section: "Characters", visible: when("charset", "custom")),
        .slider("cell", "Cell Size", 2, 40, 8, step: 0.5, curve: 1.6, section: "Characters"),
        .slider("aspect", "Cell Height", 100, 250, 180, step: 1, unit: "%", section: "Characters"),
        .toggle("bold", "Bold", false, section: "Characters"),
        .segmented("mode", "Color", [("mono", "Mono"), ("palette", "Palette"), ("original", "Original")], "mono", section: "Color"),
        .color("fg", "Characters", RGBA(hex: "#F2F2F2"), section: "Color", visible: when("mode", "mono")),
        .color("bg", "Background", RGBA(hex: "#0A0A0A"), section: "Color"),
        ParamDef(key: "palette", label: "Palette", kind: .palette(def: PalettePresets.all.first { $0.name == "Thermal Iron" } ?? PalettePresets.oneBit),
                 section: "Color", visible: when("mode", "palette")),
        .toggle("invert", "Invert", false, section: "Tone"),
        .slider("contrast", "Contrast", -100, 100, 0, step: 1, section: "Tone"),
        .slider("brightness", "Brightness", -100, 100, 0, step: 1, section: "Tone"),
    ]

    var looks: [EffectLook] {
        [
            EffectLook(id: "terminal", name: "Green Terminal", values: ["charset": "standard", "fg": RGBA(hex: "#5CFF7A"), "bg": RGBA(hex: "#04110A")]),
            EffectLook(id: "amber", name: "Amber Terminal", values: ["charset": "detailed", "cell": 6.0, "fg": RGBA(hex: "#FFB000"), "bg": RGBA(hex: "#140A00")]),
            EffectLook(id: "typewriter", name: "Typewriter", values: ["charset": "detailed", "cell": 6.0, "invert": true,
                                                                    "fg": RGBA(hex: "#1A1A1A"), "bg": RGBA(hex: "#F2F2F2")]),
            EffectLook(id: "matrix", name: "Matrix", values: ["charset": "matrix", "cell": 9.0, "fg": RGBA(hex: "#3BFF6E"), "bg": RGBA(hex: "#000000"),
                                                            "contrast": 40.0]),
            EffectLook(id: "color", name: "Color Text", values: ["charset": "detailed", "cell": 5.0, "mode": "original", "bg": RGBA(hex: "#000000")]),
            EffectLook(id: "blocks", name: "Block Art", values: ["charset": "blocks", "cell": 7.0, "aspect": 100.0, "mode": "original"]),
            EffectLook(id: "braille", name: "Braille Fine", values: ["charset": "braille", "cell": 4.0, "aspect": 150.0]),
            EffectLook(id: "binary", name: "Binary Code", values: ["charset": "binary", "cell": 8.0, "fg": RGBA(hex: "#29ADFF"), "bg": RGBA(hex: "#020812")]),
            EffectLook(id: "thermal", name: "Thermal Text", values: ["charset": "standard", "mode": "palette", "cell": 7.0,
                                                                   "palette": PalettePresets.all.first { $0.name == "Thermal Iron" } ?? PalettePresets.oneBit]),
            EffectLook(id: "yellow", name: "EFFECTOR", values: ["charset": "letters", "cell": 8.0, "fg": RGBA(hex: "#FFBB00"), "bg": RGBA(hex: "#0A0A0A")]),
        ]
    }

    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        // whole render pixels per cell so every glyph is drawn identically
        let cw = max(2, ctx.px(p.num("cell", 8)).rounded())
        let ch = max(2, (cw * p.num("aspect", 180) / 100).rounded())
        let gw = max(1, Int((Double(input.width) / cw).rounded(.up)))
        let gh = max(1, Int((Double(input.height) / ch).rounded(.up)))
        let grid = ctx.resize(input, gw, gh)
        var set = AsciiEffect.charsets.first { $0.0 == p.choice("charset", "standard") }?.2 ?? " .:-=+*#%@"
        if p.choice("charset", "standard") == "custom" {
            // unique characters, always including a blank so dark areas can stay empty
            var seen = Set<Character>()
            let chars = (" " + p.text("customChars", " .oO@")).filter { $0 != "\n" && seen.insert($0).inserted }
            set = chars.count >= 2 ? String(chars) : " .oO@"
        }
        let (atlas, n) = GlyphAtlas.shared.atlas(chars: set, bold: p.bool("bold"))
        let modeIdx: Float = ["mono": 0, "palette": 1, "original": 2][p.choice("mode", "mono")] ?? 0
        var pal = p.palette("palette") ?? PalettePresets.oneBit
        pal.colors.sort { $0.luma < $1.luma }
        var ex: [Float] = [Float(pal.colors.count)]
        for c in pal.colors { ex += c.floats }
        let u: [Float] = [Float(cw), Float(ch), Float(n), modeIdx]
            + p.color("fg", RGBA(hex: "#F2F2F2")).floats + p.color("bg", RGBA(hex: "#0A0A0A")).floats
            + [p.bool("invert") ? 1 : 0, p.f("contrast"), p.f("brightness")]
        return ctx.apply("k_ascii", [grid, atlas], u, extra: ex, w: input.width, h: input.height)
    }
}
