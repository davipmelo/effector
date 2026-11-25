// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec2 d = vec2(u[0], u[1]) * u[2] / size;
        // weighted maximum: a point light draws a full-strength tapering line,
        // while large flat bright areas are not amplified
        vec3 mx = textureLod(sampler2D(a, sLin), uv, 0.0).rgb;
        vec3 avg = mx;
        float wsum = 1.0;
        for (int i = 1; i <= 8; i++) {
            float w = exp(-float(i) * u[2] / max(u[3], 1e-3));
            vec3 s1 = textureLod(sampler2D(a, sLin), uv + d * float(i), 0.0).rgb, s2 = textureLod(sampler2D(a, sLin), uv - d * float(i), 0.0).rgb;
            mx = max(mx, max(s1, s2) * w);
            avg += (s1 + s2) * w;
            wsum += 2.0 * w;
        }
        imageStore(o, ivec2(gid), vec4(mix(mx, avg / wsum, u[4] > 0.5 ? 1.0 : 0.3), 1.0));
}
