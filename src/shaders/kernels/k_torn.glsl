// Torn paper edges (Frames & Borders, 0.9.1). The tear line of each side is procedural noise
// (seed), so no two tears are alike.
// u: 0 mode (0 torn photo, 1 paper frame), 1 fill (0 colour, 1 transparent, 2 paper), 2..4 colour,
//    5..8 sides on (top, right, bottom, left), 9..12 insets px (top, right, bottom, left),
//    13 tear width px, 14 raggedness px, 15 wavelength px, 16 fibre band px, 17 shadow, 18 shadow size px,
//    19 seed, 20 unit (px per reference px), 21 has paper, 22 paper aspect, 23 grain, 24 paper tint strength
// textures: a=1 (photo) paper=2 mean=3 (paper texture averaged to one pixel)
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D paper;
layout(set = 0, binding = 9) uniform texture2D mean;

float edgeNoise(float t, float k) {
    float L = max(u[15], 1.0);
    float sd = u[19];
    float n = (fbm(vec2(t / L, k * 7.31), sd, 4) - 0.5) * 2.0 * u[13];
    n += (vnoise(vec2(t / (L * 0.09), k * 3.1 + 20.0), sd + 3.0) - 0.5) * 2.0 * u[14];
    n += (vnoise(vec2(t / (L * 0.022), k * 5.7 + 40.0), sd + 9.0) - 0.5) * u[14] * 0.6;
    return n;
}

// signed distance to the tear (px, > 0 inside the piece); side index of the nearest edge in k
float tearDist(vec2 p, vec2 S, out float k, out float t) {
    float reach = u[13] + u[14] * 1.6;
    float d = 1e9; k = -1.0; t = 0.0;
    if (u[5] > 0.5) { float e = p.y - (u[9] + reach + edgeNoise(p.x, 0.0)); if (e < d) { d = e; k = 0.0; t = p.x; } }
    if (u[6] > 0.5) { float e = (S.x - p.x) - (u[10] + reach + edgeNoise(p.y, 1.0)); if (e < d) { d = e; k = 1.0; t = p.y; } }
    if (u[7] > 0.5) { float e = (S.y - p.y) - (u[11] + reach + edgeNoise(p.x, 2.0)); if (e < d) { d = e; k = 2.0; t = p.x; } }
    if (u[8] > 0.5) { float e = p.x - (u[12] + reach + edgeNoise(p.y, 3.0)); if (e < d) { d = e; k = 3.0; t = p.y; } }
    return d;
}

void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    ivec2 sz = imageSize(o);
    if (gid.x >= uint(sz.x) || gid.y >= uint(sz.y)) return;
    vec2 S = vec2(sz);
    vec2 p = vec2(gid) + 0.5;
    float unit = max(u[20], 1e-3);
    vec4 src = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    float k, t;
    float d = tearDist(p, S, k, t);
    if (k < 0.0) { imageStore(o, ivec2(gid), src); return; }
    // the white core of the paper shows along the tear, wider here and there
    float wf = u[16] * (0.3 + 0.95 * vnoise(vec2(t / max(u[15] * 0.14, 1.0), k * 11.0 + 60.0), u[19] + 5.0));
    // loose fibres sticking out of the torn edge
    float hair = (vnoise(p / (unit * 0.7), u[19] + 11.0) - 0.5) * u[16] * 0.7 + (hash13(vec3(floor(p / max(unit * 0.5, 0.5)), u[19])) - 0.5) * u[16] * 0.25;
    vec3 core = vec3(0.965, 0.955, 0.93) * (0.93 + 0.07 * vnoise(p / (unit * 1.3), u[19] + 2.0));
    // what lies outside the tear: colour or paper texture, with a little grain
    vec3 fill = vec3(u[2], u[3], u[4]);
    vec2 tq = p / unit;
    fill *= 1.0 + ((fbm(tq / 1.3, u[19], 3) - 0.5) * 0.08 + (fbm(tq / 90.0, u[19] + 4.0, 3) - 0.5) * 0.05) * u[23];
    if (u[21] > 0.5) {
        float ia = S.x / S.y, ta = max(u[22], 1e-4);
        vec2 uv = p / S;
        vec2 cs = ta > ia ? vec2(ia / ta, 1.0) : vec2(1.0, ta / ia);
        vec2 puv = (uv - 0.5) * cs + 0.5;
        float l = luma(textureLod(sampler2D(paper, sMir), puv, 0.0).rgb);
        float m = luma(texelFetch(sampler2D(mean, sNear), ivec2(0), 0).rgb);
        fill *= clamp(1.0 + (l - m) * u[24], 0.45, 1.35);
    }
    bool transparent = u[1] > 0.5 && u[1] < 1.5;
    float shAmt = u[17];
    float shSize = max(u[18], 0.5);
    vec3 img = src.rgb;
    vec4 outc;
    if (u[0] < 0.5) {
        // torn photo: the print ends a little before the paper does (white core), background around
        float piece = smoothstep(-0.7, 0.7, d + hair);
        float print_ = smoothstep(-0.7, 0.7, d - wf);
        vec3 pc = mix(core, img, print_);
        float k2, t2;
        float ds = tearDist(p - vec2(0.6, 1.0) * shSize * 0.5, S, k2, t2);
        float sh = ds > 0.0 ? shAmt : shAmt * exp(ds / shSize);
        sh *= 0.65;
        if (transparent) {
            float al = piece + (1.0 - piece) * sh;
            outc = vec4(al > 1e-4 ? pc * piece / al : vec3(0.0), al * src.a);
        } else {
            outc = vec4(mix(fill * (1.0 - sh), pc, piece), src.a);
        }
    } else {
        // paper frame: the paper around the photo is torn open, its white core along the tear
        float photo = smoothstep(-0.7, 0.7, d - hair);
        float surf = smoothstep(-0.7, 0.7, -d - wf);
        float sh = d > 0.0 ? shAmt * exp(-d / shSize) * 0.6 : 0.0;
        vec3 ph = img * (1.0 - sh);
        if (transparent) {
            float al = photo + (1.0 - photo) * (1.0 - surf);
            vec3 c = photo * ph + (1.0 - photo) * (1.0 - surf) * core;
            outc = vec4(al > 1e-4 ? c / al : vec3(0.0), al * src.a);
        } else {
            outc = vec4(mix(mix(core, fill, surf), ph, photo), src.a);
        }
    }
    imageStore(o, ivec2(gid), outc);
}
