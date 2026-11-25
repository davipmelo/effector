// ==== common ====

    float luma(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }

    vec3 toLinear(vec3 c) {
        c = max(c, 0.0);
        return mix(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, lessThanEqual(c, vec3(0.04045)));
    }
    vec3 toSRGB(vec3 c) {
        c = max(c, 0.0);
        return mix(1.055 * pow(c, vec3(1.0/2.4)) - 0.055, c * 12.92, lessThanEqual(c, vec3(0.0031308)));
    }
    /// Perceptual neutral grey: luminance computed in linear light, returned gamma-encoded.
    float neutralGray(vec3 c) {
        float y = dot(toLinear(c), vec3(0.2126, 0.7152, 0.0722));
        return toSRGB(vec3(y)).x;
    }

    vec3 rgb2hsl(vec3 c) {
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
        return vec3(h, s, l);
    }
    float hue2rgb(float p, float q, float t) {
        t = fract(t);
        if (t < 1.0/6.0) return p + (q - p) * 6.0 * t;
        if (t < 0.5) return q;
        if (t < 2.0/3.0) return p + (q - p) * (2.0/3.0 - t) * 6.0;
        return p;
    }
    vec3 hsl2rgb(vec3 h) {
        if (h.y < 1e-5) return vec3(h.z);
        float q = h.z < 0.5 ? h.z * (1.0 + h.y) : h.z + h.y - h.z * h.y;
        float p = 2.0 * h.z - q;
        return vec3(hue2rgb(p, q, h.x + 1.0/3.0), hue2rgb(p, q, h.x), hue2rgb(p, q, h.x - 1.0/3.0));
    }

    // OKLab (Björn Ottosson). Input/output linear sRGB.
    float cbrtf(float x) { return sign(x) * pow(abs(x), 1.0 / 3.0); }
    vec3 lin2oklab(vec3 c) {
        float l = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
        float m = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
        float s = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;
        l = cbrtf(l); m = cbrtf(m); s = cbrtf(s);
        return vec3(0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
                      1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
                      0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s);
    }
    vec3 oklab2lin(vec3 c) {
        float l = c.x + 0.3963377774 * c.y + 0.2158037573 * c.z;
        float m = c.x - 0.1055613458 * c.y - 0.0638541728 * c.z;
        float s = c.x - 0.0894841775 * c.y - 1.2914855480 * c.z;
        l = l * l * l; m = m * m * m; s = s * s * s;
        return vec3( 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
                      -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
                      -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s);
    }
    /// sRGB (gamma) → OKLCh (L, C, h in radians)
    vec3 srgb2oklch(vec3 c) {
        vec3 lab = lin2oklab(toLinear(clamp(c, 0.0, 1.0)));
        return vec3(lab.x, length(lab.yz), atan(lab.z, lab.y));
    }
    /// OKLCh → sRGB (gamma), mapping out-of-gamut colours by reducing chroma (keeps L and hue).
    vec3 oklch2srgb(vec3 lch) {
        float L = clamp(lch.x, 0.0, 1.0);
        vec2 dir = vec2(cos(lch.z), sin(lch.z));
        vec3 rgb = oklab2lin(vec3(L, dir * lch.y));
        if (all(greaterThanEqual(rgb, vec3(-1e-4))) && all(lessThanEqual(rgb, vec3(1.0001)))) return toSRGB(clamp(rgb, 0.0, 1.0));
        float lo = 0.0, hi = lch.y;
        for (int i = 0; i < 12; i++) {
            float mid = 0.5 * (lo + hi);
            vec3 t = oklab2lin(vec3(L, dir * mid));
            if (all(greaterThanEqual(t, vec3(-1e-4))) && all(lessThanEqual(t, vec3(1.0001)))) lo = mid; else hi = mid;
        }
        return toSRGB(clamp(oklab2lin(vec3(L, dir * lo)), 0.0, 1.0));
    }

    float hash12(vec2 p) {
        vec3 p3 = fract(vec3(p.xyx) * 0.1031);
        p3 += dot(p3, p3.yzx + 33.33);
        return fract((p3.x + p3.y) * p3.z);
    }
    float hash13(vec3 p3) {
        p3 = fract(p3 * 0.1031);
        p3 += dot(p3, p3.zyx + 31.32);
        return fract((p3.x + p3.y) * p3.z);
    }
    float vnoise(vec2 p, float seed) {
        vec2 i = floor(p), f = fract(p);
        vec2 w = f * f * (3.0 - 2.0 * f);
        float a = hash13(vec3(i, seed));
        float b = hash13(vec3(i + vec2(1, 0), seed));
        float c = hash13(vec3(i + vec2(0, 1), seed));
        float d = hash13(vec3(i + vec2(1, 1), seed));
        return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
    }
    // Approximate gaussian noise from two uniforms
    float gnoise(vec2 p, float seed) {
        float a = hash13(vec3(p, seed));
        float b = hash13(vec3(p + 17.31, seed + 3.7));
        return sqrt(-2.0 * log(max(a, 1e-6))) * cos(6.2831853 * b);
    }

    vec3 blendSoftLight(vec3 b, vec3 s) {
        vec3 d = mix(sqrt(b), ((16.0 * b - 12.0) * b + 4.0) * b, lessThanEqual(b, vec3(0.25)));
        return mix(b + (2.0 * s - 1.0) * (d - b), b - (1.0 - 2.0 * s) * b * (1.0 - b), lessThanEqual(s, vec3(0.5)));
    }
    vec3 setLum(vec3 c, float l) {
        float d = l - luma(c);
        c += d;
        float L = luma(c);
        float n = min(c.r, min(c.g, c.b));
        float x = max(c.r, max(c.g, c.b));
        if (n < 0.0) c = L + (c - L) * L / max(L - n, 1e-5);
        if (x > 1.0) c = L + (c - L) * (1.0 - L) / max(x - L, 1e-5);
        return c;
    }
    float satOf(vec3 c) { return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)); }
    vec3 setSat(vec3 c, float s) {
        float mx = max(c.r, max(c.g, c.b)), mn = min(c.r, min(c.g, c.b));
        float d = mx - mn;
        if (d < 1e-5) return vec3(0.0);
        return (c - mn) / d * s;
    }
    vec3 blendMode(vec3 b, vec3 s, int m) {
        switch (m) {
            case 1: return b * s;
            case 2: return 1.0 - (1.0 - b) * (1.0 - s);
            case 3: return mix(1.0 - 2.0 * (1.0 - b) * (1.0 - s), 2.0 * b * s, lessThanEqual(b, vec3(0.5)));
            case 4: return blendSoftLight(b, s);
            case 5: return mix(1.0 - 2.0 * (1.0 - b) * (1.0 - s), 2.0 * b * s, lessThanEqual(s, vec3(0.5)));
            case 6: return min(b, s);
            case 7: return max(b, s);
            case 8: return abs(b - s);
            case 9: return setLum(s, luma(b));
            case 10: return setLum(b, luma(s));
            case 11: return mix(min(vec3(1.0), b / max(1.0 - s, vec3(1e-4))), vec3(1.0), greaterThanEqual(s, vec3(1.0)));
            case 12: return mix(1.0 - min(vec3(1.0), (1.0 - b) / max(s, vec3(1e-4))), vec3(0.0), lessThanEqual(s, vec3(0.0)));
            case 13: return min(b + s, 1.0);
            case 14: return max(b + s - 1.0, 0.0);
            case 15: return b + s - 2.0 * b * s;
            case 16: return setLum(setSat(s, satOf(b)), luma(b));
            case 17: return setLum(setSat(b, satOf(s)), luma(b));
            default: return s;
        }
    }

    /// Nearest colour in a palette stored in `ex` (count, then rgb triplets).
    vec3 nearestPalette(vec3 c, int count) {
        float best = 1e9; vec3 bc = c;
        for (int i = 0; i < count; i++) {
            vec3 p = vec3(ex[1 + i*3], ex[2 + i*3], ex[3 + i*3]);
            vec3 d = c - p;
            float dist = dot(d * d, vec3(0.299, 0.587, 0.114));
            if (dist < best) { best = dist; bc = p; }
        }
        return bc;
    }
// ==== coreKernels ====

    // Integer block upscale: each source texel becomes a pix×pix block. u[0] = pix
    
    // Unpremultiply (after loading through CoreGraphics).

    // u[0] = opacity, u[1] = blend mode
    
    // Composite a straight-alpha overlay over base.
    
    // rgb from a, alpha from b

    // u[0] = levels
    
    // u[0] = threshold, u[1] = softness, u[2..4] dark, u[5..7] light
    
    // Pixelate: sample from a small texture with nearest
// ==== AdjustShaders ====
    // u: exposure, contrast, highlights, shadows, whites, blacks, temp, tint, vibrance, saturation

    // u: 8 bands x (hue, sat, lum) in -100..100. Works in OKLCh so changes stay smooth and neutrals untouched.

    // u: hue(deg), sat, light, colorize, colorHue(deg), colorSat

    // u: reds, yellows, greens, cyans, blues, magentas (percent), contrast
    
// ==== DitherShaders ====
    // Pre-pass: brightness, contrast, gamma; mono → neutral grey. u: brightness, contrast, mono, gamma

    const float bayer2[4] = float[](0, 2, 3, 1);
    const float bayer4[16] = float[](0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5);
    float bayer8v(uvec2 p) {
        uint x = p.x % 8, y = p.y % 8;
        uint v = 0;
        uint xy = x ^ y;
        for (uint b = 0; b < 3; b++) {
            v = (v << 2) | (((xy >> b) & 1u) << 1) | ((x >> b) & 1u);
        }
        return float(v);
    }
    float ditherThreshold(uvec2 p, int kind, float seed) {
        switch (kind) {
            case 0: return (bayer2[(p.y % 2) * 2 + (p.x % 2)] + 0.5) / 4.0;
            case 1: return (bayer4[(p.y % 4) * 4 + (p.x % 4)] + 0.5) / 16.0;
            case 2: return (bayer8v(p) + 0.5) / 64.0;
            case 3: return 0.5;
            case 4: return hash13(vec3(vec2(p), seed));
            case 5: { // interleaved gradient noise (blue-noise like)
                vec2 q = vec2(p) + seed * 5.588238;
                return fract(52.9829189 * fract(0.06711056 * q.x + 0.00583715 * q.y));
            }
            case 6: { // horizontal lines
                return (float(p.y % 4) + 0.5) / 4.0;
            }
            case 7: { // crosshatch-ish cluster dot 4x4
                const float cl[16] = float[](12, 5, 6, 13, 4, 0, 1, 7, 11, 3, 2, 8, 15, 10, 9, 14);
                return (cl[(p.y % 4) * 4 + (p.x % 4)] + 0.5) / 16.0;
            }
        }
        return 0.5;
    }

    // u: kind, mono, levels, spread, seed, dark rgb(5..7), light rgb(8..10)
    // ex: palette (count, rgb...)
    
// ==== HalftoneShaders ====
    vec2 rot2(vec2 p, float a) { float c = cos(a), s = sin(a); return vec2(c * p.x - s * p.y, s * p.x + c * p.y); }

    // coverage v in [0,1] → ink (0/1, antialiased) for local cell coords d in [-0.5,0.5]
    float dotInk(vec2 d, float v, int shape, float aa) {
        v = clamp(v, 0.0, 1.0);
        float m, t;
        switch (shape) {
            case 1: { // square
                m = max(abs(d.x), abs(d.y)) * 2.0; t = sqrt(v); break; }
            case 2: { // diamond
                m = abs(d.x) + abs(d.y);
                t = v <= 0.5 ? sqrt(v / 2.0) : 1.0 - sqrt((1.0 - v) / 2.0); break; }
            case 3: { // line
                m = abs(d.y) * 2.0; t = v; break; }
            case 4: { // ellipse
                m = length(d * vec2(0.78, 1.28)) * 2.0;
                t = 2.0 * sqrt(v / 3.14159); t = v > 0.7 ? mix(t, 1.6, (v - 0.7) / 0.3) : t; break; }
            case 5: { // cross
                m = min(abs(d.x), abs(d.y)) * 2.0;
                t = 1.0 - sqrt(max(0.0, 1.0 - v)); break; }
            default: { // circle
                m = length(d) * 2.0;
                t = 2.0 * sqrt(v / 3.14159); t = v > 0.785 ? mix(t, 1.42, (v - 0.785) / 0.215) : t; break; }
        }
        if (v <= 0.001) return 0.0;
        if (v >= 0.999) return 1.0;
        return 1.0 - smoothstep(t - aa, t + aa, m);
    }

    // Sample one screen at angle; returns ink amount for channel ch of a coverage function
    // u: cell(px), angle(deg), shape, mode(0 bw,1 cmyk,2 color), dotScale, softness,
    //    paper rgb (6..8), ink rgb (9..11), angC, angM, angY, angK (12..15), gcr (16)
    vec4 cmykOf(vec3 c, float gcr) {
        float k = 1.0 - max(c.r, max(c.g, c.b));
        k *= gcr;
        float d = max(1e-4, 1.0 - k);
        return vec4((1.0 - c.r - k) / d, (1.0 - c.g - k) / d, (1.0 - c.b - k) / d, k);
    }

    vec3 halftoneAt(vec2 p, texture2D src, vec2 size) {
        float cs = max(2.0, u[0]);
        int shape = int(u[2]);
        int mode = int(u[3]);
        float scale = u[4];
        float aa = max(u[5], 0.6 / cs);
        vec3 paper = vec3(u[6], u[7], u[8]);
        vec3 ink = vec3(u[9], u[10], u[11]);
        if (mode == 1) {
            float angs[4] = { u[12], u[13], u[14], u[15] };
            vec3 inks[4] = { vec3(0.0, 0.62, 0.88), vec3(0.90, 0.05, 0.52), vec3(1.0, 0.93, 0.0), vec3(0.08, 0.08, 0.09) };
            vec3 col = paper;
            for (int i = 0; i < 4; i++) {
                float a = angs[i] * 0.0174533;
                vec2 q = rot2(p, -a);
                vec2 center = (floor(q / cs) + 0.5) * cs;
                vec3 c = textureLod(sampler2D(src, sLin), rot2(center, a) / size, 0.0).rgb;
                float v = cmykOf(c, u[16])[i] * scale;
                col *= mix(vec3(1.0), inks[i], dotInk((q - center) / cs, v, shape, aa));
            }
            return col;
        }
        float a = u[1] * 0.0174533;
        vec2 q = rot2(p, -a);
        vec2 center = (floor(q / cs) + 0.5) * cs;
        vec3 c = textureLod(sampler2D(src, sLin), rot2(center, a) / size, 0.0).rgb;
        if (mode == 2) {
            float v = (1.0 - neutralGray(c) * 0.85) * scale;
            return mix(paper, c, dotInk((q - center) / cs, v, shape, aa));
        }
        float v = (1.0 - neutralGray(c)) * scale;
        return mix(paper, ink, dotInk((q - center) / cs, v, shape, aa));
    }

    // Supersampled (anti-aliased) halftone: avoids moiré between the dot grid and the pixel grid.
    
// ==== AsciiShaders ====
    // u: cellW, cellH, glyphCount, mode(0 mono,1 palette,2 original), fg rgb(4..6), bg rgb(7..9),
    //    invert, contrast, brightness, atlasCellW(px), atlasW
    
// ==== GradientShaders ====
    // u: mix, black point, white point, reverse

    // u: mix
    
// ==== GrainShaders ====
    // Film-like grain. u: amount, size(px), roughness, color, shadows, mids, highs, seed, colorAmt
    float grainNoise(vec2 p, float size, float rough, float seed) {
        vec2 q = p / max(size, 0.3);
        // smooth component (2 octaves of value noise, centered)
        float sm = (vnoise(q, seed) - 0.5) * 2.0 + (vnoise(q * 2.03 + 11.0, seed + 1.0) - 0.5) * 1.0;
        sm *= 1.25;
        // rough component: per-cell gaussian
        float rg = gnoise(floor(q), seed + 7.0) * 0.6;
        return mix(sm, rg, rough);
    }
    
// ==== GlowShaders ====
    // Highlight extraction. u: threshold, knee, tint rgb (2..4), useTint, saturation
    
    // Combine glow layers onto base. u: intensity, mode(0 screen, 1 add, 2 soft), w1, w2, w3, tint rgb(5..7), neutral
    
// ==== FilmShaders ====
    float sCurve(float x, float k) {
        x = clamp(x, 0.0, 1.0);
        float a = pow(x, k), b = pow(1.0 - x, k);
        return a / max(a + b, 1e-5);
    }
    // u: bw, sat, contrast, fade, white, gamma rgb (5..7), shadowTint (8..10), highlightTint (11..13),
    //    weights (14..16), intensity, exposure, warmth
    
// ==== TrailShaders ====
    // u: mode(0 dir,1 zoom,2 spin), angle, distance(px), copies, decay, blend, rgbShift, cx, cy, opacity
    
// ==== VHSShaders ====
    vec3 rgb2yiq(vec3 c) {
        return vec3(dot(c, vec3(0.299, 0.587, 0.114)), dot(c, vec3(0.596, -0.274, -0.322)), dot(c, vec3(0.211, -0.523, 0.312)));
    }
    vec3 yiq2rgb(vec3 y) {
        return vec3(y.x + 0.956 * y.y + 0.621 * y.z, y.x - 0.272 * y.y - 0.647 * y.z, y.x - 1.106 * y.y + 1.703 * y.z);
    }

    // Pass 1 — tape transport: per-line jitter, wobble, tracking band and head-switching skew.
    // u: 0 seed, 1 unit, 2 tracking on, 3 jitter, 4 bandPos, 5 bandH, 6 wobble on, 7 wobbleAmt, 8 wobbleFreq,
    //    9 headSwitch on, 10 headH, 11 lineH

    // Pass 2 — signal path + tape noise + tone.
    // u: 0 seed, 1 unit, 2 lumaR, 3 sharpen, 4 chroma on, 5 bleed, 6 shift, 7 chromaNoise, 8 saturation,
    //    9 noise on, 10 noiseAmt, 11 dropouts on, 12 dropAmt, 13 scan on, 14 scanAmt, 15 scanSpacing,
    //    16 vignette on, 17 vigAmt, 18 blackLift, 19 highlight roll-off, 20 warmth, 21 tint,
    //    22 tracking on, 23 bandPos, 24 bandH, 25 jitter, 26 headSwitch on, 27 headH, 28 lineH, 29 contrast

    // Composite a premultiplied rgba8 overlay over base.
    
// ==== BlobShaders ====
    // base, overlay (premultiplied rgba8), mask (r = invert region). u: invert amount, dim outside
    
// ==== TextureShaders ====
    float fbm(vec2 p, float seed, int oct) {
        float s = 0.0, a = 0.5;
        for (int i = 0; i < oct; i++) {
            s += a * vnoise(p, seed + float(i) * 7.1);
            p = p * 2.03 + 17.1;
            a *= 0.5;
        }
        return s / (1.0 - pow(0.5, float(oct)));
    }
    float ridge(vec2 p, float seed) { return 1.0 - abs(vnoise(p, seed) * 2.0 - 1.0); }
    // Voronoi: (F1, F2, cell id hash, cell id hash 2)
    vec4 voro(vec2 x, float seed) {
        vec2 n = floor(x), f = fract(x);
        float f1 = 8.0, f2 = 8.0, id = 0.0, id2 = 0.0;
        for (int j = -1; j <= 1; j++) {
            for (int i = -1; i <= 1; i++) {
                vec2 g = vec2(i, j);
                vec2 o = vec2(hash13(vec3(n + g, seed)), hash13(vec3(n + g, seed + 1.7)));
                vec2 r = g + o - f;
                float d = dot(r, r);
                if (d < f1) { f2 = f1; f1 = d; id = hash13(vec3(n + g, seed + 3.3)); id2 = hash13(vec3(n + g, seed + 5.9)); }
                else if (d < f2) { f2 = d; }
            }
        }
        return vec4(sqrt(f1), sqrt(f2), id, id2);
    }
    // Composite a premultiplied overlay with opacity. u[0] = opacity

    // Light leak. u: 0 seed, 1 unit, 2 edge, 3 size, 4 intensity, 5..7 colA, 8..10 colB, 11 warp, 12 blend
    
// ==== LensShaders ====
    // Chromatic aberration. u: mode (0 radial, 1 linear), amount(px), angle, falloff, center x, y

    // Vignette. u: amount (-1..1), midpoint, roundness, feather, color rgb (4..6), highlight protect

    // Unsharp mask. u: amount, threshold, luminance only

    // CRT. u: curvature, mask type (0 none, 1 aperture, 2 shadow, 3 slot), mask size(px), mask strength,
    //      scanlines, scan size(px), glow, brightness, vignette, corner, flicker/noise
    
// ==== LensFilterShaders ====
    // One pass of an exponential directional streak. u: dir.x, dir.y, step(px), falloff(px)
    
    // Keep small lights, suppress large bright areas. u[0] amount
    
    // a + b * gain (small working textures). u[0] gain
    
    // Spectral combine: r from a, g from b, b from c
    
    // Ring from two blurs (difference of gaussians). u: ratio

    // Prism. u: mode (0 side,1 ring,2 full), angle, offset(px), dispersion, intensity, coverage, feather, copies, blend


    // Kaleidoscope. u: segments, rotation, cx, cy, zoom, twist, mirror, dispersion, mix, rings, sx, sy

    // Crystal facets. u: unit, cell(ref px), refract(px), dispersion, edges, seed, mix, stretch

    // Focus mix for lens blur. u: mode (0 tilt-shift band, 1 radial, 2 full), pos x, pos y, size, falloff, angle, saturation
    
    // Highlight boost before blurring (fake bokeh energy). u: boost
    
// ==== DistortShaders ====
    // Edges of distortions are filled by mirroring, so there are never black holes.

    vec3 sampleSplit(texture2D a, vec2 c, vec2 d, float split, float asp) {
        // samples r/g/b with slightly different displacement (chromatic fringing)
        vec3 o;
        o.r = textureLod(sampler2D(a, sMir), c + d * (1.0 + split) / vec2(asp, 1.0), 0.0).r;
        o.g = textureLod(sampler2D(a, sMir), c + d / vec2(asp, 1.0), 0.0).g;
        o.b = textureLod(sampler2D(a, sMir), c + d * (1.0 - split) / vec2(asp, 1.0), 0.0).b;
        return o;
    }

    // Fisheye. u: amount, zoom, circle, fringe, cx, cy, projection (0 barrel, 1 true fisheye), fov(rad), edge (0 black, 1 mirror)

    // Edge distortion: the centre stays untouched, the warp grows towards the borders.
    // u: mode, strength, protect, softness, shape(0 round,1 frame), cx, cy, fringe, seed, unit, scale(ref px), edgeBlur, darken

    // Mirror & tile. u: mode, tiles, position

    // Pixel stretch. u: direction (0 right,1 left,2 down,3 up), position, jitter, seed, unit, softness
    
// ==== BlurShaders ====
    // 0 at the protected centre, 1 towards the borders. shape: 0 round, 1 frame
    float edgeMask(vec2 uv, float asp, vec2 c, float protect, float soft, float shape) {
        vec2 q = (uv - c) * vec2(asp, 1.0);
        float d = shape < 0.5 ? length(q) / length(vec2(asp, 1.0) * 0.5) : max(abs(q.x) / (asp * 0.5), abs(q.y) / 0.5);
        return smoothstep(protect, min(1.2, protect + max(0.02, soft)), d);
    }

    // Mix a blurred version in, everywhere / on the edges / in the centre.
    // u: where (0 all,1 edges,2 centre), protect, soft, shape, cx, cy, amount

    // Radial blur (zoom / spin), one pass. u: mode(0 zoom,1 spin,2 both), amount, cx, cy, protect, soft, shape, passScale

    // Directional (motion) blur, one pass. u: dx, dy (px), centred, shutter (0 box,1 fade)

    // Edge-preserving (surface) blur. u: radius(px), threshold
    
// ==== CanvasShaders ====
    // Cover-fit sample of the image into a texture of another aspect (for blurred backgrounds).

    // Final canvas. u: imageRect x,y,w,h (px), radius(px), fill (0 color,1 texture,2 gradient), color rgb (6..8),
    //    color2 rgb (9..11), shadow, shadow size(px), bg dim
    
// ==== DigitalShaders ====
    // Early digital camera colour science.
    // u: 0 seed, 1 unit, 2 white balance (-1 cool … 1 warm), 3 saturation, 4 contrast, 5 clip, 6 fringe,
    //    7 noise, 8 chroma noise, 9 exposure

// ==== CopyShaders ====
    // Photocopy. u: 0 seed, 1 unit, 2 threshold, 3 softness, 4 toner noise, 5 streaks, 6 edge shadow, 7 uneven,
    //    8..10 toner rgb, 11..13 paper rgb, 14 background noise, 15 dither

    // Thermal print finish. u: 0 seed, 1 unit, 2..4 ink, 5..7 paper, 8 fade, 9 dead lines, 10 banding, 11 gradient fade
    
// ==== FrameShaders ====
    // Frame. u: 0..3 inner rect (px), 4 radius(px), 5..7 frame rgb, 8 texture, 9 inner shadow, 10 seed, 11 unit,
    //    12..15 source rect (uv of input used to fill the window)
