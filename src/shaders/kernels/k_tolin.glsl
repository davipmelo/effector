// sRGB → linear light (for blurs that must mix light, not gamma values).
// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec4 c = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    imageStore(o, ivec2(gid), vec4(toLinear(clamp(c.rgb, 0.0, 1.0)), 1.0));
}
