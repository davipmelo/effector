// textures: a=1 b=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D b;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        imageStore(o, ivec2(gid), vec4(texelFetch(sampler2D(a, sNear), ivec2(gid), 0).rgb, texelFetch(sampler2D(b, sNear), ivec2(gid), 0).a));
}
