// Depth-of-field blur with real bokeh: per-pixel circle of confusion from the depth map (or a
// radial / band / whole-frame focus when there is no depth), gathered on a golden-angle disc
// shaped by the aperture: round or n-blade polygon, anamorphic oval, swirl (Helios/Petzval) and
// cat's-eye vignetting toward the edges. Background never bleeds over a sharp foreground.
// u: 0 max radius px, 1 focus 0..1, 2 sharp depth range, 3 mode (0 depth, 1 radial, 2 band, 3 whole),
//    4 cx, 5 cy, 6 band angle deg, 7 squeeze (>=1 = taller oval), 8 swirl, 9 cat eye, 10 blades (0 round),
//    11 blade rotation deg, 12 highlight boost, 13 seed, 14 foreground blur scale, 15 transition
// textures: a=1 depth=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D depth;

float cocAt(vec2 uv, float d, vec2 size) {
    int mode = int(u[3]);
    float t;
    float soft = max(u[15], 0.005);
    if (mode == 0) {
        float dd = d - u[1];
        t = smoothstep(u[2], u[2] + soft, abs(dd));
        if (dd > 0.0) t *= u[14];
    } else if (mode == 1) {
        vec2 p = (uv - vec2(u[4], u[5])) * vec2(size.x / size.y, 1.0);
        t = smoothstep(u[2], u[2] + soft, length(p));
    } else if (mode == 2) {
        float an = u[6] * 0.0174533;
        vec2 p = (uv - vec2(0.5, u[5])) * vec2(size.x / size.y, 1.0);
        t = smoothstep(u[2], u[2] + soft, abs(dot(p, vec2(-sin(an), cos(an)))));
    } else {
        t = 1.0;
    }
    return t * u[0];
}

vec2 shapeOffset(float r, float th) {
    float n = u[10];
    if (n >= 3.0) {
        float rot = u[11] * 0.0174533;
        float seg = 6.2831853 / n;
        float x = mod(th - rot, seg) - seg * 0.5;
        r *= cos(seg * 0.5) / cos(x);
    }
    vec2 o = vec2(cos(th), sin(th)) * r;
    o.x /= max(u[7], 1.0);
    return o;
}

void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    float dc = texelFetch(sampler2D(depth, sNear), ivec2(gid), 0).r;
    float cc = cocAt(uv, dc, size);
    float maxR = u[0];
    if (maxR < 0.5) { imageStore(o, ivec2(gid), base); return; }
    // position in the frame for swirl / cat's eye
    vec2 pf = (uv - 0.5) * 2.0 * vec2(size.x / max(size.x, size.y), size.y / max(size.x, size.y));
    float rr = clamp(length(pf), 0.0, 1.5);
    vec2 nrm = rr > 1e-4 ? pf / rr : vec2(0.0);
    vec3 acc = base.rgb;
    float wsum = 1.0;
    const int N = 120;
    float jit = hash12(vec2(gid) + u[13]) * 6.2831853;
    for (int i = 0; i < N; i++) {
        float r = sqrt((float(i) + 0.5) / float(N));
        float th = float(i) * 2.3999632 + jit;
        vec2 o = shapeOffset(r, th);
        // cat's eye: the disc is clipped by a second, off-centre aperture near the edges
        if (u[9] > 0.0) {
            vec2 q = o + nrm * u[9] * rr * 0.9;
            if (dot(q, q) > 1.0) continue;
        }
        // swirl: discs squashed radially -> stretched along circles around the centre
        if (u[8] > 0.0) {
            vec2 orad = dot(o, nrm) * nrm;
            o = (o - orad) + orad * (1.0 - u[8] * min(rr, 1.0) * 0.8);
        }
        vec2 opx = o * maxR;
        vec2 suv = uv + opx / size;
        float ds = textureLod(sampler2D(depth, sLin), suv, 0.0).r;
        float cs = cocAt(suv, ds, size);
        // a farther sample can't spread over a nearer, sharper pixel
        if (ds < dc - 0.02) cs = min(cs, max(cc, 0.0));
        float dist = r * maxR;
        float w = clamp(cs - dist + 1.0, 0.0, 1.0);
        if (w <= 0.0) continue;
        vec3 sc = textureLod(sampler2D(a, sLin), suv, 0.0).rgb;
        float l = luma(sc);
        w *= 1.0 + u[12] * 10.0 * l * l * l * l;
        acc += sc * w;
        wsum += w;
    }
    vec3 res = acc / wsum;
    // keep the in-focus pixel exactly sharp
    float sharp = 1.0 - clamp(cc / 1.5, 0.0, 1.0);
    imageStore(o, ivec2(gid), vec4(mix(res, base.rgb, sharp), base.a));
}
