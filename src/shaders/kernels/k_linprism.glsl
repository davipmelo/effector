// Linear prism filter: parallel bands (or plain echoes) that repeat the image in steps along a direction.
// u: 0 angle deg, 1 spacing px, 2 copies, 3 decay, 4 style (0 bands, 1 echo), 5 band start (0..1 along dir),
//    6 feather, 7 dispersion, 8 amount, 9 blend (0 screen, 1 lighten, 2 normal)
// textures: a=1 soft=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D soft;
vec3 tap(vec2 uv, vec2 off, float disp) {
    return vec3(textureLod(sampler2D(soft, sMir), uv - off * (1.0 + disp), 0.0).r,
                textureLod(sampler2D(soft, sMir), uv - off, 0.0).g,
                textureLod(sampler2D(soft, sMir), uv - off * (1.0 - disp), 0.0).b);
}
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    float an = u[0] * 0.0174533;
    vec2 dir = vec2(cos(an), -sin(an));
    vec2 step = dir * u[1] / size;
    int n = int(clamp(u[2], 1.0, 12.0));
    vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    float disp = u[7] * 0.25;
    vec3 c = base.rgb;
    if (u[4] < 0.5) {
        // bands: position along the direction picks how many steps this strip is shifted
        float t = dot(uv - 0.5, dir) * 1.4142 + 0.5;                   // 0..1 across the frame
        float start = u[5];
        float bw = max((1.0 - start) / float(n), 0.01);
        float k = (t - start) / bw;
        if (k > 0.0) {
            float ki = min(floor(k) + 1.0, float(n));
            float fr = fract(k);
            float fe = max(u[6], 0.001);
            vec3 cur = tap(uv, step * ki, disp);
            vec3 prev = ki > 1.0 ? tap(uv, step * (ki - 1.0), disp) : base.rgb;
            float w = smoothstep(0.0, fe, fr);
            vec3 g = mix(prev, cur, w);
            float fade = pow(1.0 - u[3] * 0.85, ki - 1.0);
            float amt = clamp(u[8], 0.0, 1.0) * fade * smoothstep(0.0, fe, k);
            c = mix(base.rgb, g, amt);
        }
    } else {
        // echoes: every copy overlaid, each weaker
        vec3 acc = vec3(0.0);
        for (int i = 1; i <= 12; i++) {
            if (i > n) break;
            vec3 g = tap(uv, step * float(i), disp);
            float w = pow(1.0 - u[3] * 0.85, float(i - 1)) * clamp(u[8], 0.0, 1.5);
            int bm = int(u[9]);
            if (bm == 1) c = mix(c, max(c, g), clamp(w, 0.0, 1.0));
            else if (bm == 2) c = mix(c, g, clamp(w * 0.5, 0.0, 1.0));
            else c = 1.0 - (1.0 - c) * (1.0 - clamp(g * w * 0.6, 0.0, 1.0));
        }
    }
    imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), base.a));
}
