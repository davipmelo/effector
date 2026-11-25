// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        uint pix = max(1u, uint(u[0]));
        uvec2 p = min(gid / pix, uvec2(uint(textureSize(sampler2D(a, sNear), 0).x) - 1, uint(textureSize(sampler2D(a, sNear), 0).y) - 1));
        imageStore(o, ivec2(gid), texelFetch(sampler2D(a, sNear), ivec2(p), 0));
}
