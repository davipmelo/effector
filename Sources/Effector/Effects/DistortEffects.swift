import Foundation
import Metal

enum DistortShaders {
    static let source = """
    // Edges of distortions are filled by mirroring, so there are never black holes.
    constexpr sampler sMirD(coord::normalized, filter::linear, address::mirrored_repeat);

    inline float3 sampleSplit(texture2d<float> a, float2 c, float2 d, float split, float asp) {
        // samples r/g/b with slightly different displacement (chromatic fringing)
        float3 o;
        o.r = a.sample(sMirD, c + d * (1.0 + split) / float2(asp, 1.0)).r;
        o.g = a.sample(sMirD, c + d / float2(asp, 1.0)).g;
        o.b = a.sample(sMirD, c + d * (1.0 - split) / float2(asp, 1.0)).b;
        return o;
    }

    // Fisheye. u: amount, zoom, circle, fringe, cx, cy, projection (0 barrel, 1 true fisheye), fov(rad), edge (0 black, 1 mirror)
    kernel void k_fisheye(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float asp = size.x / size.y;
        float2 c = float2(u[4], u[5]);
        float2 d = (UV - c) * float2(asp, 1.0);
        float r = length(d);
        float lim = 0.5 * min(asp, 1.0);
        float2 dir = r > 1e-6 ? d / r : float2(0.0);
        float3 col;
        bool outside = false;
        for (int ch = 0; ch < 3; ch++) {
            float spread = 1.0 + (float(ch) - 1.0) * u[3];
            float sr;
            if (u[6] > 0.5) {
                // true fisheye: the radius maps to a view angle up to fov/2 (stereographic, so it
                // grows smoothly); past ~180° the photo wraps around like a mirror ball
                float th = (r / lim) * u[7] * 0.5 * spread / max(0.2, u[1]);
                float srcHalf = 0.9;              // assumed half field of view of the photo (~52°)
                sr = tan(clamp(th * 0.5, -1.45, 1.45)) / tan(srcHalf * 0.5) * lim * 0.5;
            } else {
                float k = u[0] * spread;
                float r2 = r * r;
                sr = r * (1.0 + k * r2 + k * 0.5 * r2 * r2) / max(0.05, u[1]);
            }
            float2 suv = c + dir * sr / float2(asp, 1.0);
            if (any(suv < 0.0) || any(suv > 1.0)) outside = true;
            col[ch] = a.sample(sMirD, suv)[ch];
        }
        if (outside && u[8] < 0.5 && u[2] < 0.5) col = float3(0.0);
        if (u[2] > 0.5) col *= 1.0 - smoothstep(lim * 0.99, lim, r);
        o.write(float4(col, a.read(gid).a), gid);
    }

    // Twirl. u: angle(rad), radius, cx, cy, fringe
    kernel void k_twirl(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float asp = size.x / size.y;
        float2 c = float2(u[2], u[3]);
        float2 d = (UV - c) * float2(asp, 1.0);
        float r = length(d);
        float R = max(0.01, u[1]);
        float t = clamp(1.0 - r / R, 0.0, 1.0);
        t = t * t * (3.0 - 2.0 * t);
        float an = u[0] * t;
        float cs = cos(an), sn = sin(an);
        float2 rd = float2(cs * d.x - sn * d.y, sn * d.x + cs * d.y);
        o.write(float4(sampleSplit(a, c, rd, u[4] * t, asp), a.read(gid).a), gid);
    }

    // Pinch / bulge. u: amount(-1..1), radius, cx, cy, fringe
    kernel void k_bulge(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float asp = size.x / size.y;
        float2 c = float2(u[2], u[3]);
        float2 d = (UV - c) * float2(asp, 1.0);
        float r = length(d);
        float R = max(0.01, u[1]);
        float2 nd = d;
        float w = 0.0;
        if (r < R && r > 1e-6) {
            float t = r / R;
            float e = exp(u[0] * 1.6);
            nd = d / r * R * pow(t, e);
            w = 1.0 - t;
        }
        o.write(float4(sampleSplit(a, c, nd, u[4] * w, asp), a.read(gid).a), gid);
    }

    // Waves & ripples. u: mode, amp(px), wavelength(px), phase, fringe, cx, cy, damping, angle(rad)
    kernel void k_waves(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        int mode = int(u[0]);
        float wl = max(2.0, u[2]);
        float2 disp = float2(0.0);
        float ca = cos(u[8]), sa = sin(u[8]);
        float2 rp = float2(ca * p.x + sa * p.y, -sa * p.x + ca * p.y);
        if (mode == 0) disp = float2(ca, sa) * u[1] * sin(6.2831853 * rp.y / wl + u[3]);
        else if (mode == 1) disp = float2(-sa, ca) * u[1] * sin(6.2831853 * rp.x / wl + u[3]);
        else if (mode == 2) disp = float2(sin(6.2831853 * p.y / wl + u[3]), sin(6.2831853 * p.x / wl + u[3] * 1.3)) * u[1];
        else {
            float2 c = float2(u[5], u[6]) * size;
            float2 d = p - c;
            float r = length(d);
            float damp = exp(-r / max(size.x, size.y) * u[7] * 6.0);
            disp = (r > 1e-3 ? d / r : float2(0.0)) * u[1] * sin(6.2831853 * r / wl - u[3]) * damp;
        }
        float3 col;
        for (int ch = 0; ch < 3; ch++) {
            float k = 1.0 + (float(ch) - 1.0) * u[4];
            col[ch] = a.sample(sMirD, (p + disp * k) / size)[ch];
        }
        o.write(float4(col, a.read(gid).a), gid);
    }

    // Polar warp. u: mode (0 tiny planet, 1 tunnel), rotation, zoom, twist, cx, cy
    kernel void k_polar(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float asp = size.x / size.y;
        float2 q = (UV - float2(u[4], u[5])) * float2(asp, 1.0);
        float lim = 0.5 * min(asp, 1.0);
        float r = length(q) / lim / max(0.05, u[2]);
        float th = atan2(q.x, -q.y) + u[1] + u[3] * r;
        float x = fract(th / 6.2831853 + 0.5);
        if (u[6] > 0.5) x = 1.0 - abs(x * 2.0 - 1.0);      // seamless: mirror instead of a hard seam
        float y = u[0] < 0.5 ? 1.0 - r : r;
        o.write(float4(a.sample(sMirD, float2(x, y)).rgb, a.read(gid).a), gid);
    }

    // Liquid glass. u: unit, scale(ref px), strength(px), detail, fringe, seed, frosted
    kernel void k_liquid(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 p = float2(gid) + 0.5;
        float2 q = p / max(u[0], 1e-3) / max(2.0, u[1]);
        int oct = int(clamp(u[3], 1.0, 6.0));
        float2 n = float2(fbm(q, u[5], oct), fbm(q + 5.2, u[5] + 3.0, oct)) - 0.5;
        float2 disp = n * 2.0 * u[2];
        if (u[6] > 0.5) disp += (float2(hash12(p + u[5]), hash12(p.yx + u[5] * 1.7)) - 0.5) * u[2] * 0.8;
        float3 col;
        for (int ch = 0; ch < 3; ch++) {
            float k = 1.0 + (float(ch) - 1.0) * u[4];
            col[ch] = a.sample(sMirD, (p + disp * k) / size)[ch];
        }
        o.write(float4(col, a.read(gid).a), gid);
    }

    // Mirror & tile. u: mode, tiles, position
    kernel void k_mirror(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 uv = UV;
        int mode = int(u[0]);
        float c = u[2];
        if (mode == 0) uv.x = uv.x > c ? 2.0 * c - uv.x : uv.x;           // left half reflected to the right
        else if (mode == 1) uv.x = uv.x < c ? 2.0 * c - uv.x : uv.x;      // right half reflected to the left
        else if (mode == 2) uv.y = uv.y > c ? 2.0 * c - uv.y : uv.y;      // top reflected down
        else if (mode == 3) uv.y = uv.y < c ? 2.0 * c - uv.y : uv.y;      // bottom reflected up
        else if (mode == 4) uv = 0.5 - abs(uv - 0.5);                       // four-way
        else {
            float n = max(1.0, floor(u[1]));
            float2 t = uv * n;
            float2 f = fract(t);
            float2 k = floor(t);
            if (mode == 5) f = select(f, 1.0 - f, fmod(k, 2.0) > 0.5);       // mirrored tiles
            uv = f;
        }
        o.write(float4(a.sample(sMirD, uv).rgb, a.read(gid).a), gid);
    }

    // Pixel stretch. u: direction (0 right,1 left,2 down,3 up), position, jitter, seed, unit, softness
    kernel void k_stretch(KIN, texture2d<float> a [[texture(1)]]) {
        GUARD
        float2 size = OSIZE;
        float2 uv = UV;
        int dir = int(u[0]);
        float pos = u[1];
        float2 s = uv;
        bool past;
        float lane;
        if (dir == 0) { past = uv.x > pos; lane = floor(uv.y * size.y / max(1.0, u[4] * 2.0)); }
        else if (dir == 1) { past = uv.x < pos; lane = floor(uv.y * size.y / max(1.0, u[4] * 2.0)); }
        else if (dir == 2) { past = uv.y > pos; lane = floor(uv.x * size.x / max(1.0, u[4] * 2.0)); }
        else { past = uv.y < pos; lane = floor(uv.x * size.x / max(1.0, u[4] * 2.0)); }
        if (past) {
            float j = (hash12(float2(lane, u[3])) - 0.5) * u[2] * 0.2;
            if (dir < 2) s.x = clamp(pos + j, 0.0, 1.0);
            else s.y = clamp(pos + j, 0.0, 1.0);
        }
        o.write(float4(a.sample(sLin, s).rgb, a.read(gid).a), gid);
    }
    """
}

// MARK: - Fisheye

struct FisheyeEffect: Effect {
    let id = "fisheye"
    let name = "Fisheye"
    let category = EffectCategory.distort
    var symbol: String { "circle.dotted" }
    var summary: String { "Barrel or true fisheye projection up to extreme angles, with an optional circular frame — skate-video style." }
    let params: [ParamDef] = [
        .segmented("projection", "Projection", [("barrel", "Barrel"), ("fisheye", "True Fisheye")], "barrel", section: "Lens"),
        .slider("amount", "Distortion", -100, 400, 45, step: 1, curve: 1.3, section: "Lens", visible: whenNot("projection", "fisheye")),
        .slider("fov", "Field of View", 60, 340, 180, step: 1, unit: "°", section: "Lens", visible: when("projection", "fisheye")),
        .slider("zoom", "Zoom", 20, 300, 100, step: 1, unit: "%", curve: 1.3, section: "Lens"),
        .toggle("autoZoom", "Auto Fill Frame", true, section: "Lens", visible: whenNot("projection", "fisheye")),
        .segmented("edge", "Outside", [("mirror", "Mirror"), ("black", "Black")], "mirror", section: "Lens"),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%", section: "Lens"),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%", section: "Lens"),
        .sectionSwitch("circle", section: "Circular Frame", false),
        .sectionSwitch("fringeOn", section: "Color Fringing"),
        .slider("fringe", "Amount", 0, 100, 10, step: 1, unit: "%", section: "Color Fringing"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "vx", name: "VX1000 Fisheye", values: ["amount": 70.0, "fringe": 15.0]),
            EffectLook(id: "circle", name: "Full Circle", values: ["projection": "fisheye", "fov": 190.0, "circle": true]),
            EffectLook(id: "extreme", name: "Extreme 280°", values: ["projection": "fisheye", "fov": 280.0, "fringe": 25.0]),
            EffectLook(id: "ball", name: "Mirror Ball", values: ["projection": "fisheye", "fov": 340.0, "circle": true, "fringe": 30.0]),
            EffectLook(id: "bubble", name: "Bubble", values: ["amount": 260.0, "autoZoom": false, "zoom": 160.0]),
            EffectLook(id: "mild", name: "Mild Barrel", values: ["amount": 20.0]),
            EffectLook(id: "pincushion", name: "Pincushion", values: ["amount": -40.0, "zoom": 95.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let k = Double(p.f("amount", 45) / 100 * 0.9)
        let asp = Double(input.width) / Double(input.height)
        let trueFish = p.choice("projection", "barrel") == "fisheye"
        func f(_ r: Double) -> Double { 1 + k * r * r + k * 0.5 * r * r * r * r }
        var auto = 1.0
        if !trueFish && p.bool("autoZoom", true) {
            if p.bool("circle", false) {
                auto = f(0.5 * min(asp, 1))
            } else if k > 0 {
                auto = f((asp * asp / 4 + 0.25).squareRoot())
            }
        }
        let zoom = Float(p.num("zoom", 100) / 100 * auto)
        let fringe = p.bool("fringeOn", true) ? p.f("fringe", 10) / 100 * 0.08 : 0
        return ctx.apply("k_fisheye", [input], [Float(k), zoom, p.bool("circle", false) ? 1 : 0, fringe,
                                                p.f("cx", 50) / 100, p.f("cy", 50) / 100, trueFish ? 1 : 0,
                                                p.f("fov", 180) * .pi / 180, p.choice("edge", "mirror") == "mirror" ? 1 : 0])
    }
}

// MARK: - Twirl

struct TwirlEffect: Effect {
    let id = "twirl"
    let name = "Twirl"
    let category = EffectCategory.distort
    var symbol: String { "tornado" }
    var summary: String { "Spins the image around a point, like water going down a drain." }
    let params: [ParamDef] = [
        .slider("angle", "Angle", -1080, 1080, 180, step: 1, unit: "°", curve: 1),
        .slider("radius", "Radius", 5, 150, 60, step: 1, unit: "%"),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%"),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%"),
        .slider("fringe", "Color Fringing", 0, 100, 0, step: 1, unit: "%"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_twirl", [input], [p.f("angle", 180) * .pi / 180, p.f("radius", 60) / 100, p.f("cx", 50) / 100,
                                       p.f("cy", 50) / 100, p.f("fringe") / 100 * 0.1])
    }
}

// MARK: - Pinch & bulge

struct BulgeEffect: Effect {
    let id = "bulge"
    let name = "Pinch & Bulge"
    let category = EffectCategory.distort
    var symbol: String { "circle.circle.fill" }
    var summary: String { "Inflates (bulge) or sucks in (pinch) a round area of the image." }
    let params: [ParamDef] = [
        .slider("amount", "Amount", -100, 100, 50, step: 1, unit: "%"),
        .slider("radius", "Radius", 5, 150, 45, step: 1, unit: "%"),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%"),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%"),
        .slider("fringe", "Color Fringing", 0, 100, 0, step: 1, unit: "%"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_bulge", [input], [p.f("amount", 50) / 100, p.f("radius", 45) / 100, p.f("cx", 50) / 100,
                                       p.f("cy", 50) / 100, p.f("fringe") / 100 * 0.15])
    }
}

// MARK: - Waves

struct WavesEffect: Effect {
    let id = "waves"
    let name = "Waves & Ripples"
    let category = EffectCategory.distort
    var symbol: String { "water.waves" }
    var summary: String { "Sine waves, wobbly water or circular ripples, with optional rainbow edges." }
    let params: [ParamDef] = [
        .choice("mode", "Type", [("h", "Horizontal Waves"), ("v", "Vertical Waves"), ("both", "Wobble (Both)"), ("ripple", "Ripples")],
                "h"),
        .slider("amp", "Amplitude", 0, 100, 12, step: 0.5, curve: 1.5),
        .slider("wavelength", "Wavelength", 4, 600, 90, step: 1, curve: 1.6),
        .slider("phase", "Phase", 0, 360, 0, step: 1, unit: "°"),
        .slider("angle", "Angle", -90, 90, 0, step: 1, unit: "°", visible: whenAny("mode", ["h", "v"])),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%", visible: when("mode", "ripple")),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%", visible: when("mode", "ripple")),
        .slider("damping", "Damping", 0, 100, 30, step: 1, unit: "%", visible: when("mode", "ripple")),
        .slider("fringe", "Color Fringing", 0, 100, 0, step: 1, unit: "%"),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let mode: Float = ["h": 0, "v": 1, "both": 2, "ripple": 3][p.choice("mode", "h")] ?? 0
        return ctx.apply("k_waves", [input], [mode, Float(ctx.px(p.num("amp", 12))), Float(ctx.px(p.num("wavelength", 90))),
                                              p.f("phase") * .pi / 180, p.f("fringe") / 100 * 0.5, p.f("cx", 50) / 100,
                                              p.f("cy", 50) / 100, p.f("damping", 30) / 100, p.f("angle") * .pi / 180])
    }
}

// MARK: - Polar

struct PolarEffect: Effect {
    let id = "polar"
    let name = "Tiny Planet & Tunnel"
    let category = EffectCategory.distort
    var symbol: String { "globe.americas" }
    var summary: String { "Wraps the image into a circle: a tiny planet (ground in the middle) or a tunnel (sky in the middle)." }
    let params: [ParamDef] = [
        .segmented("mode", "Mode", [("planet", "Tiny Planet"), ("tunnel", "Tunnel")], "planet"),
        .slider("rotation", "Rotation", -180, 180, 0, step: 1, unit: "°"),
        .slider("zoom", "Zoom", 20, 300, 100, step: 1, unit: "%", curve: 1.3),
        .slider("twist", "Twist", -360, 360, 0, step: 1, unit: "°"),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%"),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%"),
        .toggle("seamless", "Seamless (Mirror)", true),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_polar", [input], [p.choice("mode", "planet") == "planet" ? 0 : 1, p.f("rotation") * .pi / 180,
                                       p.f("zoom", 100) / 100, p.f("twist") * .pi / 180, p.f("cx", 50) / 100, p.f("cy", 50) / 100,
                                       p.bool("seamless", true) ? 1 : 0])
    }
}

// MARK: - Liquid glass

struct LiquidEffect: Effect {
    let id = "liquid"
    let name = "Liquid & Glass"
    let category = EffectCategory.distort
    var symbol: String { "drop" }
    var summary: String { "Organic displacement: melting liquid, rippled glass or a frosted shower door." }
    let params: [ParamDef] = [
        .slider("scale", "Scale", 4, 600, 120, step: 1, curve: 1.6),
        .slider("strength", "Strength", 0, 200, 30, step: 0.5, curve: 1.5),
        .slider("detail", "Detail", 1, 6, 3, step: 1),
        .toggle("frosted", "Frosted Glass", false),
        .slider("fringe", "Color Fringing", 0, 100, 0, step: 1, unit: "%"),
        .seed(),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "melt", name: "Melting", values: ["scale": 220.0, "strength": 60.0, "detail": 2]),
            EffectLook(id: "glass", name: "Rippled Glass", values: ["scale": 25.0, "strength": 14.0, "detail": 1, "fringe": 20.0]),
            EffectLook(id: "frosted", name: "Frosted Shower Door", values: ["scale": 60.0, "strength": 10.0, "frosted": true]),
            EffectLook(id: "acid", name: "Acid Trip", values: ["scale": 160.0, "strength": 110.0, "detail": 5, "fringe": 80.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        ctx.apply("k_liquid", [input], [Float(ctx.scale), p.f("scale", 120), Float(ctx.px(p.num("strength", 30))), p.f("detail", 3),
                                        p.f("fringe") / 100 * 0.5, p.f("seed", 1) * 4.3, p.bool("frosted") ? 1 : 0])
    }
}

// MARK: - Mirror & tile

struct MirrorEffect: Effect {
    let id = "mirror"
    let name = "Mirror & Tile"
    let category = EffectCategory.distort
    var symbol: String { "square.split.2x1" }
    var summary: String { "Reflects half of the image onto the other half, four ways, or repeats it as tiles." }
    let params: [ParamDef] = [
        .choice("mode", "Mode", [("lr", "Left → Right"), ("rl", "Right → Left"), ("tb", "Top → Bottom"), ("bt", "Bottom → Top"),
                                 ("quad", "Four-way"), ("tilesm", "Mirrored Tiles"), ("tiles", "Tiles")], "lr"),
        .slider("position", "Mirror Line", 5, 95, 50, step: 1, unit: "%", visible: whenAny("mode", ["lr", "rl", "tb", "bt"])),
        .slider("tiles", "Tiles", 1, 12, 3, step: 1, visible: whenAny("mode", ["tiles", "tilesm"])),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let mode: Float = ["lr": 0, "rl": 1, "tb": 2, "bt": 3, "quad": 4, "tilesm": 5, "tiles": 6][p.choice("mode", "lr")] ?? 0
        return ctx.apply("k_mirror", [input], [mode, p.f("tiles", 3), p.f("position", 50) / 100])
    }
}

// MARK: - Pixel stretch

struct StretchEffect: Effect {
    let id = "stretch"
    let name = "Pixel Stretch"
    let category = EffectCategory.distort
    var symbol: String { "arrow.right.to.line" }
    var summary: String { "Smears one line of pixels across the rest of the image — a classic glitch-art move." }
    let params: [ParamDef] = [
        .choice("dir", "Direction", [("right", "To the Right"), ("left", "To the Left"), ("down", "Down"), ("up", "Up")], "right"),
        .slider("position", "Start Line", 0, 100, 60, step: 0.5, unit: "%"),
        .slider("jitter", "Jitter", 0, 100, 15, step: 1, unit: "%"),
        .slider("lane", "Jitter Lane Size", 0.5, 40, 3, step: 0.5, curve: 1.6),
        .seed(),
    ]
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let dir: Float = ["right": 0, "left": 1, "down": 2, "up": 3][p.choice("dir", "right")] ?? 0
        return ctx.apply("k_stretch", [input], [dir, p.f("position", 60) / 100, p.f("jitter", 15) / 100, p.f("seed", 1) * 2.9,
                                                Float(ctx.px(p.num("lane", 3)))])
    }
}
