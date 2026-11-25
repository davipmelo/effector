// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        uvec2 p = uvec2(vec2(gid) * vec2(uint(textureSize(sampler2D(a, sNear), 0).x), uint(textureSize(sampler2D(a, sNear), 0).y)) / vec2(imageSize(o)));
        p = min(p, uvec2(uint(textureSize(sampler2D(a, sNear), 0).x) - 1, uint(textureSize(sampler2D(a, sNear), 0).y) - 1));
        imageStore(o, ivec2(gid), texelFetch(sampler2D(a, sNear), ivec2(p), 0));
}
