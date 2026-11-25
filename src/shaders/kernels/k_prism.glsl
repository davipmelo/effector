// textures: a=1 soft=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D soft;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        float an = u[1] * 0.0174533;
        vec2 dir = vec2(cos(an), -sin(an));
        int mode = int(u[0]);
        float m = 1.0;
        float cov = u[5], fe = max(0.005, u[6]);
        if (mode == 0) {
            float t = dot(uv - 0.5, dir) + 0.5;
            m = smoothstep(1.0 - cov - fe, 1.0 - cov + fe, t);
        } else if (mode == 1) {
            float r = length((uv - 0.5) * vec2(size.x / max(size.x, size.y), size.y / max(size.x, size.y))) * 1.42;
            m = smoothstep(1.0 - cov - fe, 1.0 - cov + fe, r);
        }
        vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 ghost = vec3(0.0);
        int n = int(clamp(u[7], 1.0, 4.0));
        float wsum = 0.0;
        for (int k = 1; k <= n; k++) {
            vec2 off = dir * u[2] * float(k) / size;
            float disp = u[3];
            float r = textureLod(sampler2D(soft, sLin), uv - off * (1.0 + disp), 0.0).r;
            float g = textureLod(sampler2D(soft, sLin), uv - off, 0.0).g;
            float b = textureLod(sampler2D(soft, sLin), uv - off * (1.0 - disp), 0.0).b;
            float w = pow(0.6, float(k - 1));
            ghost += vec3(r, g, b) * w;
            wsum += w;
        }
        ghost /= wsum;
        float amt = clamp(u[4] * m, 0.0, 1.5);
        vec3 c = base.rgb;
        int bm = int(u[8]);
        if (bm == 1) c = mix(c, max(c, ghost), clamp(amt, 0.0, 1.0));
        else if (bm == 2) c = mix(c, ghost, clamp(amt * 0.6, 0.0, 1.0));
        else c = 1.0 - (1.0 - c) * (1.0 - clamp(ghost * amt, 0.0, 1.0));
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), base.a));
}
