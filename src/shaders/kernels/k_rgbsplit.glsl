// textures: a=1 b=2 c=3
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D b;
layout(set = 0, binding = 9) uniform texture2D c;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        imageStore(o, ivec2(gid), vec4(textureLod(sampler2D(a, sLin), uv, 0.0).r, textureLod(sampler2D(b, sLin), uv, 0.0).g, textureLod(sampler2D(c, sLin), uv, 0.0).b, 1.0));
}
