// textures: a=1 blurred=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D blurred;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        float asp = size.x / size.y;
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec2 c = vec2(u[5], u[6]);
        vec2 q = (uv - c) * vec2(asp, 1.0);
        float r = length(q);
        vec2 dir = r > 1e-6 ? q / r : vec2(0.0);
        // 0 at the centre, 1 at the corners (round) or the frame edge (frame)
        float d = u[4] < 0.5 ? r / length(vec2(asp, 1.0) * 0.5)
                             : max(abs(q.x) / (asp * 0.5), abs(q.y) / 0.5);
        float m = smoothstep(u[2], min(1.2, u[2] + max(0.02, u[3])), d);
        float k = u[1] * m;
        int mode = int(u[0]);
        vec2 p = vec2(gid) + 0.5;
        vec3 col = vec3(0.0);
        float unit = max(u[9], 1e-3);
        for (int ch = 0; ch < 3; ch++) {
            float spread = 1.0 + (float(ch) - 1.0) * u[7] * m;
            float kk = k * spread;
            vec2 nq = q;
            vec3 acc;
            if (mode == 0) {                       // bend: fisheye-like bulge on the borders
                nq = dir * r * (1.0 - kk * 0.45 * m);
            } else if (mode == 2) {                // swirl
                float an = kk * 3.1416 * m;
                float cs = cos(an), sn = sin(an);
                nq = vec2(cs * q.x - sn * q.y, sn * q.x + cs * q.y);
            } else if (mode == 3) {                // ripple
                float wl = max(2.0, u[10] * unit) / size.y;
                nq = q + dir * sin(6.2831853 * r / wl) * kk * 0.03;
            } else if (mode == 4) {                // liquid
                vec2 nn = vec2(fbm(p / unit / max(2.0, u[10]), u[8], 3), fbm(p / unit / max(2.0, u[10]) + 5.2, u[8] + 3.0, 3)) - 0.5;
                nq = q + nn * kk * 0.12;
            } else if (mode == 5) {                // squeeze: compress the borders (shows more of the scene)
                nq = dir * r * (1.0 + kk * 0.6 * m);
            }
            if (mode == 1) {                       // stretch: radial smear outward (warp speed)
                vec3 sum = vec3(0.0);
                for (int i = 0; i < 24; i++) {
                    float t = (float(i) + hash12(p + float(i))) / 24.0;
                    vec2 sq = q * (1.0 - kk * 0.35 * t);
                    sum += textureLod(sampler2D(a, sMir), c + sq / vec2(asp, 1.0), 0.0).rgb;
                }
                acc = sum / 24.0;
            } else {
                acc = textureLod(sampler2D(a, sMir), c + nq / vec2(asp, 1.0), 0.0).rgb;
            }
            col[ch] = acc[ch];
        }
        if (u[11] > 0.0) col = mix(col, textureLod(sampler2D(blurred, sLin), uv, 0.0).rgb, clamp(u[11] * m, 0.0, 1.0));
        col *= 1.0 - u[12] * m * m;
        imageStore(o, ivec2(gid), vec4(clamp(col, 0.0, 1.0), texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
