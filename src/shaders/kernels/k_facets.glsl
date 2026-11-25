// textures: a=1 soft=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D soft;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 p = vec2(gid) + 0.5;
        vec2 q = p / max(u[0], 1e-3) / max(4.0, u[1]);
        q *= vec2(1.0 - u[7] * 0.7, 1.0 + u[7] * 0.7);
        vec4 v = voro(q, u[5]);
        vec2 dir = vec2(v.z - 0.5, v.w - 0.5) * 2.0;
        vec2 off = dir * u[2] / size;
        vec3 col;
        for (int ch = 0; ch < 3; ch++) {
            float k = 1.0 + (float(ch) - 1.0) * u[3] * 0.5;
            col[ch] = textureLod(sampler2D(soft, sMir), p / size + off * k, 0.0)[ch];
        }
        float edge = 1.0 - smoothstep(0.0, 0.06, v.y - v.x);
        col += edge * u[4] * 0.6 * (0.5 + v.z);
        col *= 1.0 + (v.w - 0.5) * 0.12 * u[4];
        vec4 base = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        imageStore(o, ivec2(gid), vec4(clamp(mix(base.rgb, col, clamp(u[6], 0.0, 1.0)), 0.0, 1.0), base.a));
}
