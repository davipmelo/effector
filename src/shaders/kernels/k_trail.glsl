// Trail & motion blur: ghost copies or a smooth trail along a path.
// u: 0 mode (0 dir, 1 zoom, 2 spin, 3 arc, 4 wave, 5 shake, 6 spiral), 1 angle deg, 2 distance px, 3 copies,
//    4 decay, 5 blend (0 lighten, 1 screen, 2 normal, 3 difference, 4 darken, 5 add), 6 rgb split,
//    7 cx, 8 cy, 9 opacity, 10 smooth, 11 both directions, 12 arc bend deg, 13 wave amplitude px,
//    14 wave cycles, 15 seed, 16 colour mode (0 off, 1 rainbow, 2 gradient), 17-19 colour A, 20-22 colour B,
//    23 lights-only threshold (0 = off), 24 rainbow cycles, 25 spiral turn deg, 26 colour strength
// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;

vec2 pathUV(vec2 uv, float k, vec2 size, int mode) {
    vec2 center = vec2(u[7], u[8]);
    float ang = u[1] * 0.0174533;
    vec2 dir = vec2(cos(ang), -sin(ang));
    float dist = u[2];
    if (mode == 1) {
        float z = 1.0 + dist / max(size.x, size.y) * 4.0 * k;
        return center + (uv - center) / z;
    }
    if (mode == 2 || mode == 6) {
        float r = (mode == 2 ? u[1] : u[25]) * 0.0174533 * k * (mode == 2 ? 0.25 : 1.0);
        vec2 d = (uv - center) * size;
        float c = cos(r), sn = sin(r);
        d = vec2(c * d.x - sn * d.y, sn * d.x + c * d.y);
        if (mode == 6) d /= 1.0 + dist / max(size.x, size.y) * 4.0 * k;
        return center + d / size;
    }
    if (mode == 3) {
        // arc: the heading turns by `bend` over the trail, like a swung camera (y-up maths, flipped to screen)
        float bend = u[12] * 0.0174533;
        float th = bend * k;
        vec2 off = abs(bend) < 1e-4 ? dir * dist * k
                 : vec2(sin(ang + th) - sin(ang), -(cos(ang) - cos(ang + th))) * dist / bend;
        return uv - off / size;
    }
    if (mode == 4) {
        vec2 perp = vec2(-dir.y, dir.x);
        vec2 off = dir * dist * k + perp * u[13] * sin(k * u[14] * 6.2831853);
        return uv - off / size;
    }
    if (mode == 5) {
        // shake: each step lands somewhere random around the path (hand-held)
        float s = floor(abs(k) * 24.0);
        vec2 j = vec2(hash12(vec2(s, u[15])), hash12(vec2(u[15], s + 7.1))) - 0.5;
        vec2 off = dir * dist * k + j * u[13] * 2.0 * min(abs(k) * 3.0, 1.0);
        return uv - off / size;
    }
    return uv - dir * dist * k / size;
}

vec3 tintAt(float t) {
    int cm = int(u[16]);
    if (cm == 0) return vec3(1.0);
    vec3 c;
    if (cm == 1) c = 0.5 + 0.5 * cos(6.2831853 * (vec3(0.0, 0.33, 0.67) + abs(t) * u[24]));
    else c = mix(vec3(u[17], u[18], u[19]), vec3(u[20], u[21], u[22]), clamp(abs(t), 0.0, 1.0));
    return mix(vec3(1.0), c * 1.35, clamp(u[26], 0.0, 1.0));
}

vec3 sampleAt(vec2 uv, float t, vec2 size, int mode) {
    vec3 s;
    for (int ch = 0; ch < 3; ch++) {
        float k = t * (1.0 + (float(ch) - 1.0) * u[6]);
        s[ch] = textureLod(sampler2D(a, sMir), pathUV(uv, k, size, mode), 0.0)[ch];
    }
    if (u[23] > 0.001) s *= smoothstep(u[23] - 0.12, u[23] + 0.05, luma(s));
    return s * tintAt(t);
}

vec3 blendIt(vec3 col, vec3 s, int bm) {
    if (bm == 0) return max(col, s);
    if (bm == 1) return 1.0 - (1.0 - col) * (1.0 - clamp(s, 0.0, 1.0));
    if (bm == 2) return s;
    if (bm == 3) return abs(col - s);
    if (bm == 4) return min(col, s);
    return col + s;
}

void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    vec3 col = base.rgb;
    int mode = int(u[0]);
    int bm = int(u[5]);
    bool both = u[11] > 0.5;
    if (u[10] > 0.5) {
        // smooth: a continuous trail (many jittered sub-samples); both directions = motion blur
        const int N = 64;
        vec3 acc = vec3(0.0);
        float wsum = 0.0;
        for (int i = 0; i < N; i++) {
            float jit = hash12(vec2(gid) + float(i) * 1.37) - 0.5;
            float t = (float(i) + 0.5 + jit * 0.9) / float(N);
            if (both) t = t * 2.0 - 1.0;
            float w = pow(max(u[4], 0.02), abs(t) * 6.0);
            acc += sampleAt(uv, t, size, mode) * w;
            wsum += w;
        }
        vec3 tr = acc / max(wsum, 1e-4);
        vec3 r = blendIt(col, tr, bm);
        imageStore(o, ivec2(gid), vec4(clamp(mix(col, r, clamp(u[9], 0.0, 1.0)), 0.0, 1.0), base.a));
        return;
    }
    int n = int(clamp(u[3], 1.0, 64.0));
    for (int i = n; i >= 1; i--) {
        float t = float(i) / float(n);
        float w = pow(u[4], float(i - 1)) * u[9];
        vec3 s = sampleAt(uv, t, size, mode);
        col = mix(col, blendIt(col, s, bm), clamp(w, 0.0, 1.0));
        if (both) {
            vec3 s2 = sampleAt(uv, -t, size, mode);
            col = mix(col, blendIt(col, s2, bm), clamp(w, 0.0, 1.0));
        }
    }
    imageStore(o, ivec2(gid), vec4(clamp(col, 0.0, 1.0), base.a));
}
