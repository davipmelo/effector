// Faceted "kaleidoscope" lens filter (a cut-glass disc in front of the lens): rings of facets around
// a clear centre, each facet showing the image displaced toward the centre (so the subject repeats),
// with soft seams and rainbow dispersion at the seams.
// u: 0 cx, 1 cy, 2 rings, 3 facets (first ring), 4 clear radius, 5 refraction, 6 feather,
//    7 dispersion, 8 edge shine, 9 rotation (rad), 10 seed, 11 amount, 12 facet tilt jitter, 13 brightness variation
// textures: a=1 soft=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D soft;

const float TAU = 6.2831853;

void facetOf(vec2 q, float rings, float nf0, float clearR, float rot, out float ring, out float idx, out float n, out float ringW) {
    float r = length(q);
    ringW = (1.5 - clearR) / max(rings, 1.0);
    ring = floor((r - clearR) / ringW);
    n = max(3.0, floor(nf0 * (1.0 + ring * 0.5)));
    float th = atan(q.y, q.x) - rot + ring * 0.5 * TAU / n;
    idx = floor(fract(th / TAU) * n);
}

vec3 sampleFacet(vec2 uv, vec2 q, vec2 c, float ring, float idx, float n, float ringW, vec2 size, float sc) {
    float clearR = u[4];
    float rc = clearR + (ring + 0.5) * ringW;
    float thc = (idx + 0.5) / n * TAU + u[9] - ring * 0.5 * TAU / n;
    vec2 pc = vec2(cos(thc), sin(thc)) * rc;                          // facet centre (normalised units)
    float h = hash12(vec2(ring * 13.1 + u[10], idx * 7.3));
    float h2 = hash12(vec2(idx * 3.1 + u[10] * 1.7, ring * 5.9));
    vec2 jit = (vec2(h, h2) - 0.5) * u[12] * 0.25;
    vec2 off = (pc * u[5] + jit) * sc / size;                         // back toward the centre
    vec3 col;
    for (int ch = 0; ch < 3; ch++) {
        float k = 1.0 + (float(ch) - 1.0) * u[7] * 0.18;
        col[ch] = textureLod(sampler2D(soft, sMir), uv - off * k, 0.0)[ch];
    }
    col *= 1.0 + (h - 0.5) * u[13];
    return col;
}

void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    float sc = min(size.x, size.y) * 0.5;                             // normalised unit = half the short side
    vec2 c = vec2(u[0], u[1]) * size;
    vec2 q = (vec2(gid) + 0.5 - c) / sc;
    vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    float r = length(q);
    float clearR = u[4];
    // clear centre, softly faded in
    float fe = max(u[6], 0.002);
    float centreW = smoothstep(clearR - fe * 0.6, clearR + fe * 0.6, r);
    if (centreW <= 0.0) { imageStore(o, ivec2(gid), base); return; }
    float ring, idx, n, ringW;
    facetOf(q, u[2], u[3], clearR, u[9], ring, idx, n, ringW);
    vec3 own = sampleFacet(uv, q, c, ring, idx, n, ringW, size, sc);
    // distance to the nearest seam (angular and radial), in normalised units
    float th = atan(q.y, q.x) - u[9] + ring * 0.5 * TAU / n;
    float f = fract(th / TAU) * n;
    float da = min(fract(f), 1.0 - fract(f)) * TAU / n * r;
    float rr = (r - clearR) / ringW;
    float dr = min(fract(rr), 1.0 - fract(rr)) * ringW;
    vec3 nb;
    float d;
    if (da < dr) {
        float ni = fract(f) < 0.5 ? idx - 1.0 : idx + 1.0;
        nb = sampleFacet(uv, q, c, ring, mod(ni, n), n, ringW, size, sc);
        d = da;
    } else {
        float nr = fract(rr) < 0.5 ? ring - 1.0 : ring + 1.0;
        if (nr < 0.0) nb = base.rgb;
        else {
            float n2 = max(3.0, floor(u[3] * (1.0 + nr * 0.5)));
            float th2 = atan(q.y, q.x) - u[9] + nr * 0.5 * TAU / n2;
            float i2 = floor(fract(th2 / TAU) * n2);
            nb = sampleFacet(uv, q, c, nr, i2, n2, ringW, size, sc);
        }
        d = dr;
    }
    float w = smoothstep(0.0, fe, d);
    vec3 col = mix(mix(nb, own, 0.5), own, w);
    // a thin glint and a rainbow tint right on the seams (glass edges catch light, never a web of lines)
    float seam = 1.0 - smoothstep(0.0, fe * 0.35 + 0.0025, d);
    vec3 rainbow = 0.55 + 0.45 * cos(TAU * (vec3(0.0, 0.33, 0.67) + d / max(fe, 1e-3) * 0.7 + idx * 0.13));
    col *= mix(vec3(1.0), rainbow * 1.25, seam * clamp(u[7], 0.0, 1.0) * 0.6);
    col += seam * seam * u[8] * 0.12 * luma(col);
    float amt = clamp(u[11], 0.0, 1.0) * centreW;
    imageStore(o, ivec2(gid), vec4(clamp(mix(base.rgb, col, amt), 0.0, 1.0), base.a));
}
