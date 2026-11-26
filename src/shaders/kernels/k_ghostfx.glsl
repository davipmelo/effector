// Ghost filter (in-camera prism / "ghost" glass): part of the light goes through an angled glass
// zone and lands displaced, so a faint copy (or a smeared trail, like shutter drag) appears over
// the image. Mixed in linear light: the copy replaces the share of light it took, so brightness
// and colour stay the same.
// u: 0 zone (0 half, 1 both sides, 2 four corners, 3 around, 4 whole frame), 1 angle deg,
//    2 distance px, 3 copies, 4 fade per copy (0..1), 5 smear (0 copies, 1 smooth trail),
//    6 clear centre (0..1), 7 feather (0..1), 8 amount (0..1), 9 center x, 10 center y,
//    11 rainbow fringe (0..1)
// textures: a=1 (direct image, sharp), gs=2 (source of the copies, softened)
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D gs;

vec3 linAt(vec2 uv) { return toLinear(clamp(textureLod(sampler2D(gs, sMir), uv, 0.0).rgb, 0.0, 1.0)); }

void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    vec2 size = vec2(imageSize(o));
    if (gid.x >= uint(size.x) || gid.y >= uint(size.y)) return;
    vec2 uv = (vec2(gid) + 0.5) / size;
    vec4 b = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    vec3 lin = toLinear(clamp(b.rgb, 0.0, 1.0));

    int zone = int(u[0] + 0.5);
    float ang = u[1] * 0.0174533;
    vec2 dir = vec2(cos(ang), -sin(ang));
    // position relative to the centre, in units of the shorter side
    vec2 p = (uv - vec2(u[9], u[10])) * size / min(size.x, size.y);
    float clear = u[6] * 0.7;
    float fe = max(u[7] * 0.6, 0.002);
    float w;
    vec2 off;
    if (zone == 0) {
        float d = dot(p, dir);
        w = smoothstep(clear - fe, clear + fe, d);
        off = dir;
    } else if (zone == 1) {
        float d = dot(p, dir);
        w = smoothstep(clear, clear + fe * 2.0, abs(d));
        off = dir * sign(d);
    } else if (zone == 2) {
        // four quadrants, each pushing its copy diagonally outward (rotated by the angle)
        vec2 q = vec2(dot(p, dir), dot(p, vec2(-dir.y, dir.x)));
        w = smoothstep(clear, clear + fe * 2.0, length(p));
        vec2 dq = normalize(vec2(sign(q.x) + 1e-4, sign(q.y) + 1e-4));
        off = dir * dq.x + vec2(-dir.y, dir.x) * dq.y;
    } else if (zone == 3) {
        float r = length(p);
        w = smoothstep(clear, clear + fe * 2.0, r);
        off = r > 1e-4 ? p / r : vec2(0.0);
    } else {
        w = 1.0;
        off = dir;
    }
    w *= clamp(u[8], 0.0, 1.0);
    if (w < 1e-4) { imageStore(o, ivec2(gid), b); return; }

    vec2 stp = off * u[2] / size;
    bool smear = u[5] > 0.5;
    int n = smear ? int(clamp(length(stp * size) / 2.0, 8.0, 64.0)) : int(clamp(u[3], 1.0, 8.0) + 0.5);
    float fade = clamp(u[4], 0.0, 0.95);
    float fr = u[11];
    vec3 ghost = vec3(0.0);
    float tw = 0.0;
    for (int i = 1; i <= 64; i++) {
        if (i > n) break;
        float t = float(i) / float(n);
        float wt = smear ? (1.0 - fade * t) : pow(1.0 - fade, float(i - 1));
        vec2 q = uv - stp * t;
        vec3 s;
        if (fr > 0.001) {
            vec2 e = stp * fr * 0.08;
            s = vec3(linAt(q - e).r, linAt(q).g, linAt(q + e).b);
        } else {
            s = linAt(q);
        }
        ghost += s * wt;
        tw += wt;
    }
    ghost /= max(tw, 1e-4);
    // a real ghost filter: the direct image keeps (1 - w) of the light, the copies get the rest
    vec3 outL = mix(lin, ghost, w * (smear ? 1.0 : float(n) / float(n + 1)));
    imageStore(o, ivec2(gid), vec4(toSRGB(clamp(outL, 0.0, 1.0)), b.a));
}
