// textures: a=1 b=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D b;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        imageStore(o, ivec2(gid), vec4(max(textureLod(sampler2D(a, sLin), uv, 0.0).rgb - textureLod(sampler2D(b, sLin), uv, 0.0).rgb * u[0], 0.0) * 2.5, 1.0));
}
