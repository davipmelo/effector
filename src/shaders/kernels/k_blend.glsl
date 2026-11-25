// textures: base=1 top=2
layout(set = 0, binding = 7) uniform texture2D base;
layout(set = 0, binding = 8) uniform texture2D top;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 b = texelFetch(sampler2D(base, sNear), ivec2(gid), 0);
        vec4 t = texelFetch(sampler2D(top, sNear), ivec2(gid), 0);
        vec3 bb = clamp(b.rgb, 0.0, 1.0), tt = clamp(t.rgb, 0.0, 1.0);
        vec3 r = blendMode(bb, tt, int(u[1]));
        // Respect alpha of the effect output (e.g. overlays).
        // tolerate resampling error on alpha (keeps B&W results exactly neutral)
        float ta = t.a >= 0.99 ? 1.0 : t.a;
        float a = u[0] * ta;
        vec3 c = a >= 0.9999 ? r : mix(b.rgb, r, a);
        imageStore(o, ivec2(gid), vec4(c, max(b.a, t.a * u[0])));
}
