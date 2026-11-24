enum ShaderLibrary {
    static var fullSource: String {
        [common, coreKernels, displayShaders,
         AdjustShaders.source, DitherShaders.source, HalftoneShaders.source, AsciiShaders.source,
         GradientShaders.source, GrainShaders.source, GlowShaders.source, FilmShaders.source,
         TrailShaders.source, VHSShaders.source, BlobShaders.source,
         TextureShaders.source, LensShaders.source, LensFilterShaders.source, DistortShaders.source, DigitalShaders.source, CopyShaders.source, FrameShaders.source].joined(separator: "\n")
    }

    static let common = """
    #include <metal_stdlib>
    using namespace metal;

    #define KIN texture2d<float, access::write> o [[texture(0)]], constant float* u [[buffer(0)]], constant float* ex [[buffer(1)]], uint2 gid [[thread_position_in_grid]]
    #define GUARD if (gid.x >= o.get_width() || gid.y >= o.get_height()) return;
    #define UV ((float2(gid) + 0.5) / float2(o.get_width(), o.get_height()))
    #define OSIZE float2(o.get_width(), o.get_height())

    constexpr sampler sLin(coord::normalized, filter::linear, address::clamp_to_edge);
    constexpr sampler sNear(coord::normalized, filter::nearest, address::clamp_to_edge);
    constexpr sampler sRep(coord::normalized, filter::linear, address::repeat);

    inline float luma(float3 c) { return dot(c, float3(0.2126, 0.7152, 0.0722)); }

    inline float3 toLinear(float3 c) {
        c = max(c, 0.0);
        return select(pow((c + 0.055) / 1.055, 2.4), c / 12.92, c <= 0.04045);
    }
    inline float3 toSRGB(float3 c) {
        c = max(c, 0.0);
        return select(1.055 * pow(c, 1.0/2.4) - 0.055, c * 12.92, c <= 0.0031308);
    }
    /// Perceptual neutral grey: luminance computed in linear light, returned gamma-encoded.
    inline float neutralGray(float3 c) {
        float y = dot(toLinear(c), float3(0.2126, 0.7152, 0.0722));
        return toSRGB(float3(y)).x;
    }

    inline float3 rgb2hsl(float3 c) {
        float mx = max(c.r, max(c.g, c.b)), mn = min(c.r, min(c.g, c.b));
        float l = (mx + mn) * 0.5;
        float h = 0, s = 0;
        float d = mx - mn;
        if (d > 1e-5) {
            s = l > 0.5 ? d / (2.0 - mx - mn) : d / (mx + mn);
            if (mx == c.r) h = (c.g - c.b) / d + (c.g < c.b ? 6.0 : 0.0);
            else if (mx == c.g) h = (c.b - c.r) / d + 2.0;
            else h = (c.r - c.g) / d + 4.0;
            h /= 6.0;
        }
        return float3(h, s, l);
    }
    inline float hue2rgb(float p, float q, float t) {
        t = fract(t);
        if (t < 1.0/6.0) return p + (q - p) * 6.0 * t;
        if (t < 0.5) return q;
        if (t < 2.0/3.0) return p + (q - p) * (2.0/3.0 - t) * 6.0;
        return p;
    }
    inline float3 hsl2rgb(float3 h) {
        if (h.y < 1e-5) return float3(h.z);
        float q = h.z < 0.5 ? h.z * (1.0 + h.y) : h.z + h.y - h.z * h.y;
        float p = 2.0 * h.z - q;
        return float3(hue2rgb(p, q, h.x + 1.0/3.0), hue2rgb(p, q, h.x), hue2rgb(p, q, h.x - 1.0/3.0));
    }

    // OKLab (Björn Ottosson). Input/output linear sRGB.
    inline float cbrtf(float x) { return sign(x) * pow(abs(x), 1.0 / 3.0); }
    inline float3 lin2oklab(float3 c) {
        float l = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
        float m = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
        float s = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;
        l = cbrtf(l); m = cbrtf(m); s = cbrtf(s);
        return float3(0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
                      1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
                      0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s);
    }
    inline float3 oklab2lin(float3 c) {
        float l = c.x + 0.3963377774 * c.y + 0.2158037573 * c.z;
        float m = c.x - 0.1055613458 * c.y - 0.0638541728 * c.z;
        float s = c.x - 0.0894841775 * c.y - 1.2914855480 * c.z;
        l = l * l * l; m = m * m * m; s = s * s * s;
        return float3( 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
                      -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
                      -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s);
    }
    /// sRGB (gamma) → OKLCh (L, C, h in radians)
    inline float3 srgb2oklch(float3 c) {
        float3 lab = lin2oklab(toLinear(clamp(c, 0.0, 1.0)));
        return float3(lab.x, length(lab.yz), atan2(lab.z, lab.y));
    }
    /// OKLCh → sRGB (gamma), mapping out-of-gamut colours by reducing chroma (keeps L and hue).
    inline float3 oklch2srgb(float3 lch) {
        float L = clamp(lch.x, 0.0, 1.0);
        float2 dir = float2(cos(lch.z), sin(lch.z));
        float3 rgb = oklab2lin(float3(L, dir * lch.y));
        if (all(rgb >= -1e-4) && all(rgb <= 1.0001)) return toSRGB(clamp(rgb, 0.0, 1.0));
        float lo = 0.0, hi = lch.y;
        for (int i = 0; i < 12; i++) {
            float mid = 0.5 * (lo + hi);
            float3 t = oklab2lin(float3(L, dir * mid));
            if (all(t >= -1e-4) && all(t <= 1.0001)) lo = mid; else hi = mid;
        }
        return toSRGB(clamp(oklab2lin(float3(L, dir * lo)), 0.0, 1.0));
    }

    inline float hash12(float2 p) {
        float3 p3 = fract(float3(p.xyx) * 0.1031);
        p3 += dot(p3, p3.yzx + 33.33);
        return fract((p3.x + p3.y) * p3.z);
    }
    inline float hash13(float3 p3) {
        p3 = fract(p3 * 0.1031);
        p3 += dot(p3, p3.zyx + 31.32);
        return fract((p3.x + p3.y) * p3.z);
    }
    inline float vnoise(float2 p, float seed) {
        float2 i = floor(p), f = fract(p);
        float2 w = f * f * (3.0 - 2.0 * f);
        float a = hash13(float3(i, seed));
        float b = hash13(float3(i + float2(1, 0), seed));
        float c = hash13(float3(i + float2(0, 1), seed));
        float d = hash13(float3(i + float2(1, 1), seed));
        return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
    }
    // Approximate gaussian noise from two uniforms
    inline float gnoise(float2 p, float seed) {
        float a = hash13(float3(p, seed));
        float b = hash13(float3(p + 17.31, seed + 3.7));
        return sqrt(-2.0 * log(max(a, 1e-6))) * cos(6.2831853 * b);
    }

    inline float3 blendSoftLight(float3 b, float3 s) {
        float3 d = select(sqrt(b), ((16.0 * b - 12.0) * b + 4.0) * b, b <= 0.25);
        return select(b + (2.0 * s - 1.0) * (d - b), b - (1.0 - 2.0 * s) * b * (1.0 - b), s <= 0.5);
    }
    inline float3 setLum(float3 c, float l) {
        float d = l - luma(c);
        c += d;
        float L = luma(c);
        float n = min(c.r, min(c.g, c.b));
        float x = max(c.r, max(c.g, c.b));
        if (n < 0.0) c = L + (c - L) * L / max(L - n, 1e-5);
        if (x > 1.0) c = L + (c - L) * (1.0 - L) / max(x - L, 1e-5);
        return c;
    }
    inline float satOf(float3 c) { return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)); }
    inline float3 setSat(float3 c, float s) {
        float mx = max(c.r, max(c.g, c.b)), mn = min(c.r, min(c.g, c.b));
        float d = mx - mn;
        if (d < 1e-5) return float3(0.0);
        return (c - mn) / d * s;
    }
    inline float3 blendMode(float3 b, float3 s, int m) {
        switch (m) {
            case 1: return b * s;
            case 2: return 1.0 - (1.0 - b) * (1.0 - s);
            case 3: return select(1.0 - 2.0 * (1.0 - b) * (1.0 - s), 2.0 * b * s, b <= 0.5);
            case 4: return blendSoftLight(b, s);
            case 5: return select(1.0 - 2.0 * (1.0 - b) * (1.0 - s), 2.0 * b * s, s <= 0.5);
            case 6: return min(b, s);
            case 7: return max(b, s);
            case 8: return abs(b - s);
            case 9: return setLum(s, luma(b));
            case 10: return setLum(b, luma(s));
            case 11: return select(min(1.0, b / max(1.0 - s, 1e-4)), float3(1.0), s >= 1.0);
            case 12: return select(1.0 - min(1.0, (1.0 - b) / max(s, 1e-4)), float3(0.0), s <= 0.0);
            case 13: return min(b + s, 1.0);
            case 14: return max(b + s - 1.0, 0.0);
            case 15: return b + s - 2.0 * b * s;
            case 16: return setLum(setSat(s, satOf(b)), luma(b));
            case 17: return setLum(setSat(b, satOf(s)), luma(b));
            default: return s;
        }
    }

    /// Nearest colour in a palette stored in `ex` (count, then rgb triplets).
    inline float3 nearestPalette(float3 c, constant float* ex, int count) {
        float best = 1e9; float3 bc = c;
        for (int i = 0; i < count; i++) {
            float3 p = float3(ex[1 + i*3], ex[2 + i*3], ex[3 + i*3]);
            float3 d = c - p;
            float dist = dot(d * d, float3(0.299, 0.587, 0.114));
            if (dist < best) { best = dist; bc = p; }
        }
        return bc;
    }
    """

    static let coreKernels = """
    kernel void k_copy(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        o.write(a.read(gid), gid);
    }
    kernel void k_clamp(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        o.write(clamp(a.read(gid), 0.0, 1.0), gid);
    }
    kernel void k_nearest(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        uint2 p = uint2(float2(gid) * float2(a.get_width(), a.get_height()) / OSIZE);
        p = min(p, uint2(a.get_width() - 1, a.get_height() - 1));
        o.write(a.read(p), gid);
    }
    // Integer block upscale: each source texel becomes a pix×pix block. u[0] = pix
    kernel void k_block(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        uint pix = max(1u, uint(u[0]));
        uint2 p = min(gid / pix, uint2(a.get_width() - 1, a.get_height() - 1));
        o.write(a.read(p), gid);
    }
    // Unpremultiply (after loading through CoreGraphics).
    kernel void k_unpremul(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 c = a.read(gid);
        if (c.a > 1e-5) c.rgb /= c.a;
        o.write(clamp(c, 0.0, 1.0), gid);
    }
    kernel void k_premul(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 c = clamp(a.read(gid), 0.0, 1.0);
        c.rgb *= c.a;
        o.write(c, gid);
    }
    // u[0] = opacity, u[1] = blend mode
    kernel void k_blend(KIN, texture2d<float> base [[texture(1)]], texture2d<float> top [[texture(2)]]) {
        GUARD
        float4 b = base.read(gid);
        float4 t = top.read(gid);
        float3 bb = clamp(b.rgb, 0.0, 1.0), tt = clamp(t.rgb, 0.0, 1.0);
        float3 r = blendMode(bb, tt, int(u[1]));
        // Respect alpha of the effect output (e.g. overlays).
        // tolerate resampling error on alpha (keeps B&W results exactly neutral)
        float ta = t.a >= 0.99 ? 1.0 : t.a;
        float a = u[0] * ta;
        float3 c = a >= 0.9999 ? r : mix(b.rgb, r, a);
        o.write(float4(c, max(b.a, t.a * u[0])), gid);
    }
    // Composite a straight-alpha overlay over base.
    kernel void k_over(KIN, texture2d<float> base [[texture(1)]], texture2d<float> top [[texture(2)]]) {
        GUARD
        float4 b = base.read(gid);
        float4 t = top.sample(sLin, UV);
        o.write(float4(mix(b.rgb, t.rgb, t.a), max(b.a, t.a)), gid);
    }
    // rgb from a, alpha from b
    kernel void k_withalpha(KIN, texture2d<float> a [[texture(1)]], texture2d<float> b [[texture(2)]]) {
        GUARD
        o.write(float4(a.read(gid).rgb, b.read(gid).a), gid);
    }
    kernel void k_invert(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 c = a.read(gid);
        o.write(float4(1.0 - c.rgb, c.a), gid);
    }
    // u[0] = levels
    kernel void k_posterize(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 c = a.read(gid);
        float n = max(2.0, u[0]) - 1.0;
        o.write(float4(floor(c.rgb * n + 0.5) / n, c.a), gid);
    }
    // u[0] = threshold, u[1] = softness, u[2..4] dark, u[5..7] light
    kernel void k_threshold(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float4 c = a.read(gid);
        float l = neutralGray(c.rgb);
        float s = max(u[1], 0.0005);
        float t = smoothstep(u[0] - s, u[0] + s, l);
        o.write(float4(mix(float3(u[2], u[3], u[4]), float3(u[5], u[6], u[7]), t), c.a), gid);
    }
    // Pixelate: sample from a small texture with nearest
    """

    static let displayShaders = """
    struct DispOut { float4 pos [[position]]; float2 uv; };
    // u: viewW, viewH, imgX, imgY, imgW, imgH (in drawable pixels, origin top-left), checker size, dark
    vertex DispOut v_display(uint vid [[vertex_id]], constant float* u [[buffer(0)]]) {
        float2 corners[4] = { float2(0,0), float2(1,0), float2(0,1), float2(1,1) };
        float2 c = corners[vid];
        float2 p = float2(u[2], u[3]) + c * float2(u[4], u[5]);
        float2 ndc = float2(p.x / u[0] * 2.0 - 1.0, 1.0 - p.y / u[1] * 2.0);
        DispOut o; o.pos = float4(ndc, 0, 1); o.uv = c; return o;
    }
    fragment float4 f_display(DispOut in [[stage_in]], texture2d<float> t [[texture(0)]], constant float* u [[buffer(0)]]) {
        constexpr sampler sm(coord::normalized, filter::linear, address::clamp_to_edge);
        constexpr sampler sn(coord::normalized, filter::nearest, address::clamp_to_edge);
        float4 c = u[7] > 0.5 ? t.sample(sn, in.uv) : t.sample(sm, in.uv);
        float2 cell = floor(in.pos.xy / u[6]);
        float ch = fmod(cell.x + cell.y, 2.0) < 1.0 ? 0.28 : 0.36;
        float3 bg = float3(ch);
        return float4(mix(bg, clamp(c.rgb, 0.0, 1.0), clamp(c.a, 0.0, 1.0)), 1.0);
    }
    """
}
