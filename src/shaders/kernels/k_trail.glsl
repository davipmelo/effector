// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 col = base.rgb;
        int mode = int(u[0]);
        int n = int(clamp(u[3], 1.0, 48.0));
        float ang = u[1] * 0.0174533;
        vec2 dir = vec2(cos(ang), -sin(ang)) * u[2] / size;
        vec2 center = vec2(u[7], u[8]);
        if (u[10] > 0.5) {
            // smooth: a continuous trail (many sub-samples), optionally centred = motion blur
            const int N = 64;
            vec3 acc = vec3(0.0); float wsum = 0.0;
            for (int i = 0; i < N; i++) {
                float jit = hash12(vec2(gid) + float(i) * 1.37) - 0.5;
                float t = (float(i) + 0.5 + jit * 0.9) / float(N);
                if (u[11] > 0.5) t = t * 2.0 - 1.0;
                float w = pow(max(u[4], 0.02), abs(t) * 6.0);
                vec3 s;
                for (int ch = 0; ch < 3; ch++) {
                    float k = t * (1.0 + (float(ch) - 1.0) * u[6]);
                    vec2 suv;
                    if (mode == 1) {
                        float z = 1.0 + u[2] / max(size.x, size.y) * 4.0 * k;
                        suv = center + (uv - center) / z;
                    } else if (mode == 2) {
                        float r = u[1] * 0.0174533 * k * 0.25;
                        vec2 d = (uv - center) * size;
                        float c = cos(r), sn = sin(r);
                        suv = center + vec2(c * d.x - sn * d.y, sn * d.x + c * d.y) / size;
                    } else {
                        suv = uv - dir * k;
                    }
                    s[ch] = textureLod(sampler2D(a, sLin), suv, 0.0)[ch];
                }
                acc += s * w; wsum += w;
            }
            vec3 tr = acc / max(wsum, 1e-4);
            int bm = int(u[5]);
            vec3 r;
            if (bm == 0) r = max(col, tr);
            else if (bm == 1) r = 1.0 - (1.0 - col) * (1.0 - tr);
            else if (bm == 2) r = tr;
            else if (bm == 3) r = abs(col - tr);
            else r = min(col, tr);
            imageStore(o, ivec2(gid), vec4(clamp(mix(col, r, clamp(u[9], 0.0, 1.0)), 0.0, 1.0), base.a));
            return;
        }
        for (int i = n; i >= 1; i--) {
            float t = float(i) / float(n);
            float w = pow(u[4], float(i - 1)) * u[9];
            vec3 s;
            for (int ch = 0; ch < 3; ch++) {
                float k = t * (1.0 + (float(ch) - 1.0) * u[6]);
                vec2 suv;
                if (mode == 1) {
                    float z = 1.0 + u[2] / max(size.x, size.y) * 4.0 * k;
                    suv = center + (uv - center) / z;
                } else if (mode == 2) {
                    float r = u[1] * 0.0174533 * k * 0.25;
                    vec2 d = (uv - center) * size;
                    float c = cos(r), sn = sin(r);
                    d = vec2(c * d.x - sn * d.y, sn * d.x + c * d.y);
                    suv = center + d / size;
                } else {
                    suv = uv - dir * k;
                }
                s[ch] = textureLod(sampler2D(a, sLin), suv, 0.0)[ch];
            }
            int bm = int(u[5]);
            vec3 r;
            if (bm == 0) r = max(col, s);
            else if (bm == 1) r = 1.0 - (1.0 - col) * (1.0 - s);
            else if (bm == 2) r = s;
            else if (bm == 3) r = abs(col - s);
            else r = min(col, s);
            col = mix(col, r, clamp(w, 0.0, 1.0));
        }
        imageStore(o, ivec2(gid), vec4(clamp(col, 0.0, 1.0), base.a));
}
