// textures: a=1 bright=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D bright;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 p = vec2(gid) + 0.5;
        float unit = max(u[1], 1e-3);
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 c = clamp(s.rgb, 0.0, 1.0);
        c = toSRGB(toLinear(c) * exp2(u[9]));
        float wb = u[2];
        c *= vec3(1.0 + 0.08 * wb, 1.0 + 0.01 * wb, 1.0 - 0.12 * wb);
        // punchy tone curve with a hard digital clip
        float k = u[4];
        c = (c - 0.45) * k + 0.45;
        float clip = mix(1.0, 0.86, u[5]);
        c = clamp(c / clip, 0.0, 1.0);
        float l = luma(c);
        c = mix(vec3(l), c, u[3]);
        // purple fringing around blown highlights
        float b = textureLod(sampler2D(bright, sLin), ((vec2(gid) + 0.5) / vec2(imageSize(o))), 0.0).r;
        float fr = max(0.0, b - smoothstep(0.75, 1.0, l)) * u[6];
        c = mix(c, c * vec3(0.75, 0.45, 1.0) + vec3(0.25, 0.0, 0.45) * fr, clamp(fr * 1.6, 0.0, 0.85));
        // sensor noise: luma grain + blotchy chroma, strongest in the shadows
        float sh = 1.0 - smoothstep(0.0, 0.7, l);
        vec2 q = p / unit;
        float n = gnoise(vec2(gid), u[0]) * u[7] * 0.05 * (0.4 + sh);
        vec2 cn = vec2(vnoise(q * 0.45, u[0] + 3.0) - 0.5, vnoise(q * 0.45 + 9.0, u[0] + 5.0) - 0.5);
        c += n;
        c += vec3(cn.x, -0.5 * (cn.x + cn.y), cn.y) * u[8] * 0.16 * sh;
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), s.a));
}
