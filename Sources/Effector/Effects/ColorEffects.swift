import Foundation
import Metal

enum GradientShaders {
    static let source = """
    // u: mix, black point, white point, reverse
    kernel void k_gradmap(KIN, texture2d<float> a [[texture(1)]], texture2d<float> lut [[texture(2)]]) {
        GUARD
        float4 s = a.read(gid);
        float g = neutralGray(clamp(s.rgb, 0.0, 1.0));
        g = clamp((g - u[1]) / max(1e-3, u[2] - u[1]), 0.0, 1.0);
        if (u[3] > 0.5) g = 1.0 - g;
        float n = float(lut.get_width());
        float3 c = lut.sample(sLin, float2((g * (n - 1.0) + 0.5) / n, 0.5)).rgb;
        o.write(float4(mix(s.rgb, c, u[0]), s.a), gid);
    }

    // u: mix
    kernel void k_lut3d(KIN, texture2d<float> a [[texture(1)]], texture3d<float> lut [[texture(2)]]) {
        GUARD
        float4 s = a.read(gid);
        float n = float(lut.get_width());
        float3 c = clamp(s.rgb, 0.0, 1.0);
        float3 uvw = c * (n - 1.0) / n + 0.5 / n;
        constexpr sampler s3(coord::normalized, filter::linear, address::clamp_to_edge);
        float3 r = lut.sample(s3, uvw).rgb;
        o.write(float4(mix(s.rgb, r, u[0]), s.a), gid);
    }
    """
}

struct GradientMapEffect: Effect {
    let id = "gradientmap"
    let name = "Gradient Map"
    let category = EffectCategory.color
    var symbol: String { "thermometer.medium" }
    let params: [ParamDef] = [
        ParamDef(key: "gradient", label: "Gradient", kind: .gradient(def: GradientPresets.thermalIron), section: "Gradient"),
        .slider("amount", "Amount", 0, 100, 100, unit: "%", section: "Gradient"),
        .slider("black", "Input Black", 0, 100, 0, unit: "%", section: "Range"),
        .slider("white", "Input White", 0, 100, 100, unit: "%", section: "Range"),
        .toggle("reverse", "Reverse", false, section: "Range"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let g = p.gradient("gradient") ?? GradientPresets.thermalIron
        let lut = ctx.gpu.makeLUT1D((0..<256).map { i in
            let c = g.sample(Double(i) / 255)
            return SIMD4(Float(c.r), Float(c.g), Float(c.b), 1)
        })
        return ctx.apply("k_gradmap", [input, lut], [p.f("amount", 100) / 100, p.f("black") / 100, p.f("white", 100) / 100,
                                                     p.bool("reverse") ? 1 : 0])
    }
}

/// Cache of 3D LUT textures (parsing/uploading is expensive).
final class LUTCache {
    static let shared = LUTCache()
    private var cache: [Int: MTLTexture] = [:]
    private let lock = NSLock()

    func texture(_ lut: LUTData) -> MTLTexture {
        let key = lut.hashValue
        lock.lock(); defer { lock.unlock() }
        if let t = cache[key] { return t }
        let n = lut.size
        let d = MTLTextureDescriptor()
        d.textureType = .type3D
        d.pixelFormat = .rgba32Float
        d.width = n; d.height = n; d.depth = n
        d.usage = [.shaderRead]
        d.storageMode = .shared
        let t = GPU.shared.device.makeTexture(descriptor: d)!
        var rgba = [Float](repeating: 1, count: n * n * n * 4)
        for i in 0..<(n * n * n) {
            rgba[i * 4] = lut.values[i * 3]; rgba[i * 4 + 1] = lut.values[i * 3 + 1]; rgba[i * 4 + 2] = lut.values[i * 3 + 2]
        }
        rgba.withUnsafeBytes { b in
            t.replace(region: MTLRegionMake3D(0, 0, 0, n, n, n), mipmapLevel: 0, slice: 0,
                      withBytes: b.baseAddress!, bytesPerRow: n * 16, bytesPerImage: n * n * 16)
        }
        if cache.count > 8 { cache.removeAll() }
        cache[key] = t
        return t
    }
}

struct LUTEffect: Effect {
    let id = "lut"
    let name = "LUT (.cube)"
    let category = EffectCategory.color
    var symbol: String { "cube" }
    let params: [ParamDef] = [
        ParamDef(key: "lut", label: "LUT", kind: .lut),
        .slider("amount", "Amount", 0, 100, 100, unit: "%"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        guard let lut = p.lut("lut"), !lut.isEmpty else { return input }
        return ctx.apply("k_lut3d", [input, LUTCache.shared.texture(lut)], [p.f("amount", 100) / 100])
    }
}
