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

    // Edge distortion: the centre stays untouched, the warp grows towards the borders.
    // u: mode, strength, protect, softness, shape(0 round,1 frame), cx, cy, fringe, seed, unit, scale(ref px), edgeBlur, darken
    kernel void k_edgewarp(KIN, texture2d<float> a [[texture(1)]], texture2d<float> blurred [[texture(2)]]) {
        GUARD
        float2 size = OSIZE;
        float asp = size.x / size.y;
        float2 uv = UV;
        float2 c = float2(u[5], u[6]);
        float2 q = (uv - c) * float2(asp, 1.0);
        float r = length(q);
        float2 dir = r > 1e-6 ? q / r : float2(0.0);
        // 0 at the centre, 1 at the corners (round) or the frame edge (frame)
        float d = u[4] < 0.5 ? r / length(float2(asp, 1.0) * 0.5)
                             : max(abs(q.x) / (asp * 0.5), abs(q.y) / 0.5);
        float m = smoothstep(u[2], min(1.2, u[2] + max(0.02, u[3])), d);
        float k = u[1] * m;
        int mode = int(u[0]);
        float2 p = float2(gid) + 0.5;
        float3 col = float3(0.0);
        float unit = max(u[9], 1e-3);
        for (int ch = 0; ch < 3; ch++) {
            float spread = 1.0 + (float(ch) - 1.0) * u[7] * m;
            float kk = k * spread;
            float2 nq = q;
            float3 acc;
            if (mode == 0) {                       // bend: fisheye-like bulge on the borders
                nq = dir * r * (1.0 - kk * 0.45 * m);
            } else if (mode == 2) {                // swirl
                float an = kk * 3.1416 * m;
                float cs = cos(an), sn = sin(an);
                nq = float2(cs * q.x - sn * q.y, sn * q.x + cs * q.y);
            } else if (mode == 3) {                // ripple
                float wl = max(2.0, u[10] * unit) / size.y;
                nq = q + dir * sin(6.2831853 * r / wl) * kk * 0.03;
            } else if (mode == 4) {                // liquid
                float2 nn = float2(fbm(p / unit / max(2.0, u[10]), u[8], 3), fbm(p / unit / max(2.0, u[10]) + 5.2, u[8] + 3.0, 3)) - 0.5;
                nq = q + nn * kk * 0.12;
            } else if (mode == 5) {                // squeeze: compress the borders (shows more of the scene)
                nq = dir * r * (1.0 + kk * 0.6 * m);
            }
            if (mode == 1) {                       // stretch: radial smear outward (warp speed)
                float3 sum = float3(0.0);
                for (int i = 0; i < 24; i++) {
                    float t = (float(i) + hash12(p + float(i))) / 24.0;
                    float2 sq = q * (1.0 - kk * 0.35 * t);
                    sum += a.sample(sMirD, c + sq / float2(asp, 1.0)).rgb;
                }
                acc = sum / 24.0;
            } else {
                acc = a.sample(sMirD, c + nq / float2(asp, 1.0)).rgb;
            }
            col[ch] = acc[ch];
        }
        if (u[11] > 0.0) col = mix(col, blurred.sample(sLin, uv).rgb, clamp(u[11] * m, 0.0, 1.0));
        col *= 1.0 - u[12] * m * m;
        o.write(float4(clamp(col, 0.0, 1.0), a.read(gid).a), gid);
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
        .sectionSwitch("edgeBlurOn", section: "Edge Blur", false),
        .slider("edgeBlur", "Amount", 0, 100, 70, step: 1, unit: "%", section: "Edge Blur"),
        .slider("edgeBlurRadius", "Radius", 1, 80, 14, step: 0.5, curve: 1.6, section: "Edge Blur"),
        .slider("edgeBlurCenter", "Sharp Center Size", 0, 90, 40, step: 1, unit: "%", section: "Edge Blur"),
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
        let out = ctx.apply("k_fisheye", [input], [Float(k), zoom, p.bool("circle", false) ? 1 : 0, fringe,
                                                   p.f("cx", 50) / 100, p.f("cy", 50) / 100, trueFish ? 1 : 0,
                                                   p.f("fov", 180) * .pi / 180, p.choice("edge", "mirror") == "mirror" ? 1 : 0])
        guard p.bool("edgeBlurOn", false) else { return out }
        let b = ctx.blur(out, sigma: ctx.px(p.num("edgeBlurRadius", 14)))
        return ctx.apply("k_maskmix", [out, b], [1, p.f("edgeBlurCenter", 40) / 100, 0.5, 0, p.f("cx", 50) / 100, p.f("cy", 50) / 100,
                                                 p.f("edgeBlur", 70) / 100])
    }
}

// MARK: - Edge distort

struct EdgeDistortEffect: Effect {
    let id = "edgewarp"
    let name = "Edge Distort"
    let category = EffectCategory.distort
    var symbol: String { "circle.dashed.inset.filled" }
    var summary: String { "Keeps the centre intact and warps only the borders: bend, stretch, swirl, ripple, liquid or squeeze." }
    let params: [ParamDef] = [
        .choice("mode", "Warp", [("bend", "Bend (Fisheye Edges)"), ("stretch", "Stretch (Warp Speed)"), ("swirl", "Swirl"),
                                 ("ripple", "Ripple"), ("liquid", "Liquid"), ("squeeze", "Squeeze")], "bend", section: "Warp"),
        .slider("strength", "Strength", -100, 200, 50, step: 1, unit: "%", section: "Warp"),
        .slider("scale", "Pattern Size", 4, 400, 60, step: 1, curve: 1.6, section: "Warp", visible: whenAny("mode", ["ripple", "liquid"])),
        .seed(section: "Warp"),
        .segmented("shape", "Shape", [("round", "Round"), ("frame", "Frame")], "round", section: "Protected Center"),
        .slider("protect", "Center Size", 0, 90, 35, step: 1, unit: "%", section: "Protected Center"),
        .slider("softness", "Transition", 2, 100, 45, step: 1, unit: "%", section: "Protected Center"),
        .slider("cx", "Center X", 0, 100, 50, step: 1, unit: "%", section: "Protected Center"),
        .slider("cy", "Center Y", 0, 100, 50, step: 1, unit: "%", section: "Protected Center"),
        .sectionSwitch("fringeOn", section: "Color Fringing"),
        .slider("fringe", "Amount", 0, 100, 30, step: 1, unit: "%", section: "Color Fringing"),
        .sectionSwitch("blurOn", section: "Edge Blur", false),
        .slider("blur", "Amount", 0, 100, 50, step: 1, unit: "%", section: "Edge Blur"),
        .slider("blurRadius", "Radius", 1, 80, 12, step: 0.5, curve: 1.6, section: "Edge Blur"),
        .sectionSwitch("darkOn", section: "Edge Darkening", false),
        .slider("darken", "Amount", 0, 100, 35, step: 1, unit: "%", section: "Edge Darkening"),
    ]
    var looks: [EffectLook] {
        [
            EffectLook(id: "fishedges", name: "Fisheye Edges", values: ["mode": "bend", "strength": 90.0, "protect": 30.0]),
            EffectLook(id: "warp", name: "Warp Speed", values: ["mode": "stretch", "strength": 120.0, "protect": 40.0, "fringeOn": true,
                                                              "fringe": 50.0]),
            EffectLook(id: "vortex", name: "Edge Vortex", values: ["mode": "swirl", "strength": 80.0, "protect": 35.0]),
            EffectLook(id: "dream", name: "Dream Edges", values: ["mode": "liquid", "strength": 120.0, "scale": 120.0, "blurOn": true, "blur": 40.0]),
            EffectLook(id: "tunnel", name: "Tunnel Vision", values: ["mode": "stretch", "strength": 80.0, "protect": 25.0, "blurOn": true,
                                                                   "darkOn": true, "darken": 50.0]),
            EffectLook(id: "ripple", name: "Rippled Frame", values: ["mode": "ripple", "shape": "frame", "strength": 100.0, "scale": 40.0,
                                                                   "protect": 55.0]),
            EffectLook(id: "squeeze", name: "Wide Squeeze", values: ["mode": "squeeze", "strength": 100.0, "protect": 30.0]),
        ]
    }
    func apply(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext) -> MTLTexture {
        let mode: Float = ["bend": 0, "stretch": 1, "swirl": 2, "ripple": 3, "liquid": 4, "squeeze": 5][p.choice("mode", "bend")] ?? 0
        let blurOn = p.bool("blurOn", false)
        let blurred = blurOn ? ctx.blur(input, sigma: ctx.px(p.num("blurRadius", 12))) : input
        return ctx.apply("k_edgewarp", [input, blurred], [
            mode, p.f("strength", 50) / 100, p.f("protect", 35) / 100, p.f("softness", 45) / 100,
            p.choice("shape", "round") == "frame" ? 1 : 0, p.f("cx", 50) / 100, p.f("cy", 50) / 100,
            p.bool("fringeOn", true) ? p.f("fringe", 30) / 100 * 0.08 : 0, p.f("seed", 1) * 3.1, Float(ctx.scale), p.f("scale", 60),
            blurOn ? p.f("blur", 50) / 100 : 0, p.bool("darkOn", false) ? p.f("darken", 35) / 100 : 0,
        ])
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
