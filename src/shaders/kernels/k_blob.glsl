// textures: base=1 ov=2 mask=3
layout(set = 0, binding = 7) uniform texture2D base;
layout(set = 0, binding = 8) uniform texture2D ov;
layout(set = 0, binding = 9) uniform texture2D mask;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 b = texelFetch(sampler2D(base, sNear), ivec2(gid), 0);
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        float m = textureLod(sampler2D(mask, sLin), uv, 0.0).a;
        vec3 c = b.rgb;
        c = mix(c, 1.0 - c, m * u[0]);
        c *= 1.0 - u[1] * (1.0 - m);
        vec4 t = textureLod(sampler2D(ov, sLin), uv, 0.0);
        c = c * (1.0 - t.a) + t.rgb;
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), b.a));
}
