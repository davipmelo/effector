// textures: base=1 top=2
layout(set = 0, binding = 7) uniform texture2D base;
layout(set = 0, binding = 8) uniform texture2D top;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 b = texelFetch(sampler2D(base, sNear), ivec2(gid), 0);
        vec4 t = textureLod(sampler2D(top, sLin), ((vec2(gid) + 0.5) / vec2(imageSize(o))), 0.0);
        imageStore(o, ivec2(gid), vec4(mix(b.rgb, t.rgb, t.a), max(b.a, t.a)));
}
