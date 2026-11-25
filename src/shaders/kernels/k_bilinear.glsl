// Bilinear resample to the output size (replaces MPSImageBilinearScale).
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 uv = (vec2(gid) + 0.5) / vec2(imageSize(o));
    imageStore(o, ivec2(gid), textureLod(sampler2D(a, sLin), uv, 0.0));
}
