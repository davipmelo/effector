// Darkest channel per pixel (for Dehaze's haze estimate).
// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec3 c = clamp(texelFetch(sampler2D(a, sNear), ivec2(gid), 0).rgb, 0.0, 1.0);
    float m = min(min(c.r, c.g), c.b);
    imageStore(o, ivec2(gid), vec4(m, m, m, 1.0));
}
