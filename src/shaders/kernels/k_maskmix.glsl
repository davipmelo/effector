// textures: base=1 bl=2
layout(set = 0, binding = 7) uniform texture2D base;
layout(set = 0, binding = 8) uniform texture2D bl;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        float m = 1.0;
        int w = int(u[0]);
        if (w > 0) {
            m = edgeMask(uv, size.x / size.y, vec2(u[4], u[5]), u[1], u[2], u[3]);
            if (w == 2) m = 1.0 - m;
        }
        vec4 b = texelFetch(sampler2D(base, sNear), ivec2(gid), 0);
        imageStore(o, ivec2(gid), vec4(mix(b.rgb, textureLod(sampler2D(bl, sLin), uv, 0.0).rgb, clamp(m * u[6], 0.0, 1.0)), b.a));
}
