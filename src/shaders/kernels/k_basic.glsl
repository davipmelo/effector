// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 src = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 lin = toLinear(src.rgb) * exp2(u[0]);
        float t = u[6] / 100.0, ti = u[7] / 100.0;
        lin *= vec3(1.0 + 0.28 * t, 1.0, 1.0 - 0.28 * t);
        lin *= vec3(1.0 + 0.12 * ti, 1.0 - 0.2 * ti, 1.0 + 0.12 * ti);
        vec3 c = toSRGB(lin);
        float l = clamp(luma(c), 0.0, 1.0);
        float ws = 6.75 * l * (1.0 - l) * (1.0 - l);
        float wh = 6.75 * l * l * (1.0 - l);
        float dl = u[3] / 100.0 * 0.35 * ws + u[2] / 100.0 * 0.35 * wh
                 + u[4] / 100.0 * 0.25 * l * l + u[5] / 100.0 * 0.25 * (1.0 - l) * (1.0 - l);
        c += dl;
        float k = u[1] / 100.0;
        float cf = k >= 0.0 ? 1.0 + k * 1.5 : 1.0 + k * 0.9;
        c = (c - 0.5) * cf + 0.5;
        float g = luma(c);
        float sat = (max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)));
        float vib = u[8] / 100.0 * (1.0 - clamp(sat, 0.0, 1.0));
        c = mix(vec3(g), c, 1.0 + vib);
        c = mix(vec3(luma(c)), c, 1.0 + u[9] / 100.0);
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), src.a));
}
