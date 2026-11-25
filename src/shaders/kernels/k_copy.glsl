// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        imageStore(o, ivec2(gid), texelFetch(sampler2D(a, sNear), ivec2(gid), 0));
}
